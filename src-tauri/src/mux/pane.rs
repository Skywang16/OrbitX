//! 面板接口和实现

use std::io::{Read, Write};
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::mux::error::{PaneError, PaneResult};
use crate::mux::{PaneId, PtySize, TerminalConfig};
use portable_pty::{CommandBuilder, MasterPty, PtySize as PortablePtySize, SlavePty};

pub trait Pane: Send + Sync {
    fn pane_id(&self) -> PaneId;

    fn write(&self, data: &[u8]) -> PaneResult<()>;

    fn resize(&self, size: PtySize) -> PaneResult<()>;

    fn reader(&self) -> PaneResult<Box<dyn Read + Send>>;

    fn is_dead(&self) -> bool;

    fn mark_dead(&self);

    fn get_size(&self) -> PtySize;
}

pub struct LocalPane {
    pane_id: PaneId,
    size: Arc<Mutex<PtySize>>,
    dead: Arc<AtomicBool>,
    master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    _slave: Arc<Mutex<Box<dyn SlavePty + Send>>>,
}

impl LocalPane {
    pub fn new(pane_id: PaneId, size: PtySize) -> PaneResult<Self> {
        Self::new_with_config(pane_id, size, &TerminalConfig::default())
    }

    /// 创建新的本地面板
    pub fn new_with_config(
        pane_id: PaneId,
        size: PtySize,
        config: &TerminalConfig,
    ) -> PaneResult<Self> {
        tracing::info!(
            "创建本地面板: {:?}, 大小: {:?}, shell: {}",
            pane_id,
            size,
            config.shell_config.program
        );

        let pty_system = portable_pty::native_pty_system();

        // 转换尺寸格式
        let pty_size = PortablePtySize {
            rows: size.rows,
            cols: size.cols,
            pixel_width: size.pixel_width,
            pixel_height: size.pixel_height,
        };

        let pty_pair = pty_system.openpty(pty_size).map_err(|err| {
            PaneError::Internal(format!("Failed to create PTY for {:?}: {err}", pane_id))
        })?;

        let shell_args = &config.shell_config.args;
        tracing::debug!(
            "PTY创建成功，配置命令: {} {:?}",
            config.shell_config.program,
            shell_args
        );

        // 配置命令
        let mut cmd = CommandBuilder::new(&config.shell_config.program);
        cmd.args(shell_args);

        if let Some(cwd) = &config.shell_config.working_directory {
            cmd.cwd(cwd);
            tracing::debug!("设置工作目录: {:?}", cwd);
        }

        if let Some(env_vars) = &config.shell_config.env {
            for (key, value) in env_vars {
                cmd.env(key, value);
            }
        }

        // 添加基本环境变量
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");

        // 添加UTF-8和中文支持的环境变量
        cmd.env("TERM_PROGRAM", "OrbitX"); // <-- 激活Shell Integration的关键
        cmd.env("LANG", "en_US.UTF-8");
        cmd.env("LC_ALL", "en_US.UTF-8");
        cmd.env("LC_CTYPE", "en_US.UTF-8");

        // 自动注入Shell Integration脚本（环境变量方式）
        let shell_type = crate::shell::ShellType::from_program(&config.shell_config.program);
        tracing::info!(
            "Shell type: {:?}, supports_integration: {}",
            shell_type,
            shell_type.supports_integration()
        );
        if shell_type.supports_integration() {
            let script_generator = crate::shell::ShellScriptGenerator::default();
            if let Ok(integration_script) =
                script_generator.generate_integration_script(&shell_type)
            {
                cmd.env("ORBITX_SHELL_INTEGRATION", "1");

                // 静默注入：通过环境变量和配置文件
                match shell_type {
                    crate::shell::ShellType::Zsh => {
                        // 为zsh创建临时配置目录
                        let temp_dir =
                            std::env::temp_dir().join(format!("orbitx-{}", process::id()));
                        tracing::info!(
                            "Attempting to create temp dir for Zsh integration: {:?}",
                            temp_dir
                        );
                        if let Err(e) = std::fs::create_dir_all(&temp_dir) {
                            tracing::error!("创建临时目录失败: {}", e);
                        } else {
                            tracing::info!("Temp dir created successfully: {:?}", temp_dir);
                            let temp_zshrc = temp_dir.join(".zshrc");
                            match std::fs::File::create(&temp_zshrc) {
                                Ok(mut file) => {
                                    use std::io::Write;
                                    let _ = writeln!(
                                        file,
                                        "# Load user zshrc FIRST (so nvm etc. loads)"
                                    );
                                    let _ = writeln!(file, "[[ -f ~/.zshrc ]] && source ~/.zshrc");
                                    let _ = writeln!(file, "");
                                    let _ = writeln!(
                                        file,
                                        "# OrbitX Shell Integration (after user env loaded)"
                                    );
                                    let _ = writeln!(file, "{}", integration_script);
                                    tracing::info!(
                                        "Shell Integration script written to {:?}",
                                        temp_zshrc
                                    );

                                    if let Some(original_zdotdir) = std::env::var_os("ZDOTDIR") {
                                        cmd.env("ORBITX_ORIGINAL_ZDOTDIR", original_zdotdir);
                                    }
                                    cmd.env("ZDOTDIR", &temp_dir);
                                    tracing::info!("ZDOTDIR set to {:?}", temp_dir);
                                }
                                Err(e) => {
                                    tracing::error!("Failed to create temp .zshrc: {}", e);
                                }
                            }
                        }
                    }
                    crate::shell::ShellType::Bash => {
                        // 为bash创建临时初始化文件
                        let temp_dir =
                            std::env::temp_dir().join(format!("orbitx-{}", process::id()));
                        if let Err(e) = std::fs::create_dir_all(&temp_dir) {
                            tracing::warn!("创建临时目录失败: {}", e);
                        } else {
                            let temp_bashrc = temp_dir.join(".bashrc");
                            if let Ok(mut file) = std::fs::File::create(&temp_bashrc) {
                                use std::io::Write;
                                let _ =
                                    writeln!(file, "# Load user bashrc FIRST (so nvm etc. loads)");
                                let _ = writeln!(file, "[[ -f ~/.bashrc ]] && source ~/.bashrc");
                                let _ = writeln!(file, "");
                                let _ = writeln!(
                                    file,
                                    "# OrbitX Shell Integration (after user env loaded)"
                                );
                                let _ = writeln!(file, "{}", integration_script);

                                cmd.env("BASH_ENV", temp_bashrc);
                            }
                        }
                    }
                    crate::shell::ShellType::Fish => {
                        // Fish shell集成
                        cmd.env("ORBITX_INTEGRATION_SCRIPT", integration_script);
                    }
                    _ => {
                        // 其他shell仅设置环境变量
                        cmd.env("ORBITX_INTEGRATION_SCRIPT", integration_script);
                    }
                }
            } else {
                tracing::warn!(
                    "无法生成 {} 的Shell Integration脚本",
                    shell_type.display_name()
                );
            }
        } else {
            tracing::debug!("Shell {} 不支持自动集成", shell_type.display_name());
        }
        cmd.env("XMODIFIERS", "@im=ibus");
        cmd.env("GTK_IM_MODULE", "ibus");
        cmd.env("QT_IM_MODULE", "ibus");

        // 启动子进程并监控
        match pty_pair.slave.spawn_command(cmd) {
            Ok(_child) => {
                tracing::debug!("Shell进程启动成功");
            }
            Err(e) => {
                tracing::error!("Shell进程启动失败: {:?}", e);
                return Err(PaneError::Spawn {
                    reason: e.to_string(),
                });
            }
        }

        tracing::debug!("子进程启动成功");

        let writer = pty_pair.master.take_writer().map_err(|err| {
            PaneError::Internal(format!(
                "Failed to acquire PTY writer for {:?}: {err}",
                pane_id
            ))
        })?;

        let master = Arc::new(Mutex::new(pty_pair.master));
        let slave = Arc::new(Mutex::new(pty_pair.slave));

        tracing::info!("本地面板创建完成: {:?}", pane_id);

        Ok(Self {
            pane_id,
            size: Arc::new(Mutex::new(size)),
            dead: Arc::new(AtomicBool::new(false)),
            master,
            writer: Arc::new(Mutex::new(writer)),
            _slave: slave,
        })
    }
}

impl Pane for LocalPane {
    fn pane_id(&self) -> PaneId {
        self.pane_id
    }

    fn write(&self, data: &[u8]) -> PaneResult<()> {
        if self.is_dead() {
            tracing::debug!("尝试写入已死亡的面板: {:?}", self.pane_id);
            return Err(PaneError::PaneDead);
        }

        tracing::debug!("面板 {:?} 写入 {} 字节数据", self.pane_id, data.len());

        let mut writer = self
            .writer
            .lock()
            .map_err(|err| PaneError::from_poison("writer", err))?;

        // 使用Write trait写入数据
        use std::io::Write;
        writer.write_all(data).map_err(|err| {
            PaneError::Internal(format!("Pane {:?} PTY write failed: {err}", self.pane_id))
        })?;

        writer.flush().map_err(|err| {
            PaneError::Internal(format!("Pane {:?} PTY flush failed: {err}", self.pane_id))
        })?;

        tracing::trace!("面板 {:?} 写入完成", self.pane_id);
        Ok(())
    }

    fn resize(&self, size: PtySize) -> PaneResult<()> {
        if self.is_dead() {
            tracing::debug!("尝试调整已死亡面板的大小: {:?}", self.pane_id);
            return Err(PaneError::PaneDead);
        }

        tracing::info!("调整面板 {:?} 大小: {:?}", self.pane_id, size);

        // 更新内部尺寸记录
        {
            let mut current_size = self
                .size
                .lock()
                .map_err(|err| PaneError::from_poison("size", err))?;
            *current_size = size;
        }

        // 调整PTY大小
        let pty_size = PortablePtySize {
            rows: size.rows,
            cols: size.cols,
            pixel_width: size.pixel_width,
            pixel_height: size.pixel_height,
        };

        let master = self
            .master
            .lock()
            .map_err(|err| PaneError::from_poison("master", err))?;

        master.resize(pty_size).map_err(|err| {
            PaneError::Internal(format!("Pane {:?} PTY resize failed: {err}", self.pane_id))
        })?;

        tracing::debug!("面板 {:?} 大小调整完成", self.pane_id);
        Ok(())
    }

    fn reader(&self) -> PaneResult<Box<dyn Read + Send>> {
        if self.is_dead() {
            return Err(PaneError::PaneDead);
        }

        let master = self
            .master
            .lock()
            .map_err(|err| PaneError::from_poison("master", err))?;

        let reader = master
            .try_clone_reader()
            .map_err(|err| PaneError::Internal(format!("Failed to clone PTY reader: {err}")))?;

        Ok(reader)
    }

    fn is_dead(&self) -> bool {
        self.dead.load(Ordering::Relaxed)
    }

    fn mark_dead(&self) {
        tracing::debug!("标记面板为死亡状态: {:?}", self.pane_id);
        self.dead.store(true, Ordering::Relaxed);
    }

    fn get_size(&self) -> PtySize {
        self.size
            .lock()
            .map(|size| *size)
            .unwrap_or_else(|_| PtySize::default())
    }
}

// 便利方法实现
impl LocalPane {
    /// 写入字符串数据
    pub fn write_str(&self, data: &str) -> PaneResult<()> {
        self.write(data.as_bytes())
    }

    /// 写入带换行的字符串
    pub fn write_line(&self, data: &str) -> PaneResult<()> {
        let mut line = data.to_string();
        line.push('\n');
        self.write(line.as_bytes())
    }

    /// 发送控制字符
    pub fn send_control(&self, ctrl_char: char) -> PaneResult<()> {
        let ctrl_code = match ctrl_char {
            'c' => 0x03, // Ctrl+C
            'd' => 0x04, // Ctrl+D
            'z' => 0x1A, // Ctrl+Z
            _ => {
                return Err(PaneError::Internal(format!(
                    "Unsupported control character: {}",
                    ctrl_char
                )))
            }
        };
        self.write(&[ctrl_code])
    }

    /// 发送特殊键序列
    pub fn send_key(&self, key: &str) -> PaneResult<()> {
        let sequence: &[u8] = match key {
            "Enter" => b"\r",
            "Tab" => b"\t",
            "Backspace" => b"\x7f",
            "Escape" => b"\x1b",
            "Up" => b"\x1b[A",
            "Down" => b"\x1b[B",
            "Right" => b"\x1b[C",
            "Left" => b"\x1b[D",
            "Home" => b"\x1b[H",
            "End" => b"\x1b[F",
            "PageUp" => b"\x1b[5~",
            "PageDown" => b"\x1b[6~",
            "Delete" => b"\x1b[3~",
            "Insert" => b"\x1b[2~",
            _ => {
                return Err(PaneError::Internal(format!(
                    "Unsupported key sequence: {}",
                    key
                )))
            }
        };
        self.write(sequence)
    }
}
