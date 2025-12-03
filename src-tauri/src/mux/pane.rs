//! 面板接口和实现

use std::io::{Read, Write};
use std::process;
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
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

    /// 获取创建时使用的 shell 名称
    fn shell_name(&self) -> &str;
}

pub struct LocalPane {
    pane_id: PaneId,
    rows: AtomicU16,
    cols: AtomicU16,
    pixel_width: AtomicU16,
    pixel_height: AtomicU16,
    dead: AtomicBool,
    shell_name: String,
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
        let pty_pair = Self::create_pty(pane_id, size)?;
        let mut cmd = Self::build_command(config)?;
        Self::setup_shell_integration(&mut cmd, config)?;
        let (master, writer, slave) = Self::spawn_process(pane_id, pty_pair, cmd)?;

        Ok(Self {
            pane_id,
            rows: AtomicU16::new(size.rows),
            cols: AtomicU16::new(size.cols),
            pixel_width: AtomicU16::new(size.pixel_width),
            pixel_height: AtomicU16::new(size.pixel_height),
            dead: AtomicBool::new(false),
            shell_name: config.shell_config.program.clone(),
            master,
            writer,
            _slave: slave,
        })
    }

    /// 创建 PTY
    fn create_pty(
        pane_id: PaneId,
        size: PtySize,
    ) -> PaneResult<portable_pty::PtyPair> {
        let pty_system = portable_pty::native_pty_system();

        let pty_size = PortablePtySize {
            rows: size.rows,
            cols: size.cols,
            pixel_width: size.pixel_width,
            pixel_height: size.pixel_height,
        };

        pty_system.openpty(pty_size).map_err(|err| {
            PaneError::Internal(format!("Failed to create PTY for {:?}: {err}", pane_id))
        })
    }

    /// 构建 Shell 命令
    fn build_command(config: &TerminalConfig) -> PaneResult<CommandBuilder> {

        let mut cmd = CommandBuilder::new(&config.shell_config.program);
        cmd.args(&config.shell_config.args);

        if let Some(cwd) = &config.shell_config.working_directory {
            cmd.cwd(cwd);
        }

        if let Some(env_vars) = &config.shell_config.env {
            for (key, value) in env_vars {
                cmd.env(key, value);
            }
        }

        // 添加基本环境变量
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        cmd.env("TERM_PROGRAM", "OrbitX");
        cmd.env("LANG", "en_US.UTF-8");
        cmd.env("LC_ALL", "en_US.UTF-8");
        cmd.env("LC_CTYPE", "en_US.UTF-8");

        Ok(cmd)
    }

    /// 设置 Shell Integration
    fn setup_shell_integration(
        cmd: &mut CommandBuilder,
        config: &TerminalConfig,
    ) -> PaneResult<()> {

        let shell_type = crate::shell::ShellType::from_program(&config.shell_config.program);
        
        if !shell_type.supports_integration() {
            return Ok(());
        }

        let script_generator = crate::shell::ShellScriptGenerator::default();
        let integration_script = match script_generator.generate_integration_script(&shell_type) {
            Ok(script) => script,
            Err(_) => return Ok(()),
        };

        cmd.env("ORBITX_SHELL_INTEGRATION", "1");

        match shell_type {
            crate::shell::ShellType::Zsh => {
                Self::setup_zsh_integration(cmd, &integration_script)?;
            }
            crate::shell::ShellType::Bash => {
                Self::setup_bash_integration(cmd, &integration_script)?;
            }
            crate::shell::ShellType::Fish => {
                cmd.env("ORBITX_INTEGRATION_SCRIPT", integration_script);
            }
            _ => {
                cmd.env("ORBITX_INTEGRATION_SCRIPT", integration_script);
            }
        }

        cmd.env("XMODIFIERS", "@im=ibus");
        cmd.env("GTK_IM_MODULE", "ibus");
        cmd.env("QT_IM_MODULE", "ibus");

        Ok(())
    }

    /// 设置 Zsh Shell Integration
    fn setup_zsh_integration(
        cmd: &mut CommandBuilder,
        integration_script: &str,
    ) -> PaneResult<()> {
        let temp_dir = std::env::temp_dir().join(format!("orbitx-{}", process::id()));
        
        std::fs::create_dir_all(&temp_dir).map_err(|e| {
            PaneError::Internal(format!("创建临时目录失败: {}", e))
        })?;

        let temp_zshrc = temp_dir.join(".zshrc");
        let mut file = std::fs::File::create(&temp_zshrc).map_err(|e| {
            PaneError::Internal(format!("创建 .zshrc 失败: {}", e))
        })?;

        use std::io::Write;
        writeln!(file, "# Load user zshrc FIRST (so nvm etc. loads)").ok();
        writeln!(file, "[[ -f ~/.zshrc ]] && source ~/.zshrc").ok();
        writeln!(file).ok();
        writeln!(file, "# OrbitX Shell Integration (after user env loaded)").ok();
        writeln!(file, "{}", integration_script).ok();

        if let Some(original_zdotdir) = std::env::var_os("ZDOTDIR") {
            cmd.env("ORBITX_ORIGINAL_ZDOTDIR", original_zdotdir);
        }
        cmd.env("ZDOTDIR", &temp_dir);

        Ok(())
    }

    /// 设置 Bash Shell Integration
    fn setup_bash_integration(
        cmd: &mut CommandBuilder,
        integration_script: &str,
    ) -> PaneResult<()> {
        let temp_dir = std::env::temp_dir().join(format!("orbitx-{}", process::id()));
        
        std::fs::create_dir_all(&temp_dir).map_err(|e| {
            PaneError::Internal(format!("创建临时目录失败: {}", e))
        })?;

        let temp_bashrc = temp_dir.join(".bashrc");
        let mut file = std::fs::File::create(&temp_bashrc).map_err(|e| {
            PaneError::Internal(format!("创建 .bashrc 失败: {}", e))
        })?;

        use std::io::Write;
        writeln!(file, "# Load user bashrc FIRST (so nvm etc. loads)").ok();
        writeln!(file, "[[ -f ~/.bashrc ]] && source ~/.bashrc").ok();
        writeln!(file).ok();
        writeln!(file, "# OrbitX Shell Integration (after user env loaded)").ok();
        writeln!(file, "{}", integration_script).ok();

        cmd.env("BASH_ENV", temp_bashrc);

        Ok(())
    }

    /// 启动子进程
    fn spawn_process(
        pane_id: PaneId,
        pty_pair: portable_pty::PtyPair,
        cmd: CommandBuilder,
    ) -> PaneResult<(
        Arc<Mutex<Box<dyn MasterPty + Send>>>,
        Arc<Mutex<Box<dyn Write + Send>>>,
        Arc<Mutex<Box<dyn SlavePty + Send>>>,
    )> {

        pty_pair.slave.spawn_command(cmd).map_err(|e| {
            PaneError::Spawn {
                reason: e.to_string(),
            }
        })?;

        let writer = pty_pair.master.take_writer().map_err(|err| {
            PaneError::Internal(format!(
                "Failed to acquire PTY writer for {:?}: {err}",
                pane_id
            ))
        })?;

        let master = Arc::new(Mutex::new(pty_pair.master));
        let writer = Arc::new(Mutex::new(writer));
        let slave = Arc::new(Mutex::new(pty_pair.slave));

        Ok((master, writer, slave))
    }
}

impl Pane for LocalPane {
    fn pane_id(&self) -> PaneId {
        self.pane_id
    }

    fn write(&self, data: &[u8]) -> PaneResult<()> {
        if self.is_dead() {
            return Err(PaneError::PaneDead);
        }

        let mut writer = self
            .writer
            .lock()
            .map_err(|err| PaneError::from_poison("writer", err))?;

        // 使用Write trait写入数据
        use std::io::Write;
        writer.write_all(data).map_err(|err| {
            tracing::error!("Pane {:?} write failed: {}", self.pane_id, err);
            PaneError::Internal(format!("Pane {:?} PTY write failed: {err}", self.pane_id))
        })?;

        writer.flush().map_err(|err| {
            tracing::error!("Pane {:?} flush failed: {}", self.pane_id, err);
            PaneError::Internal(format!("Pane {:?} PTY flush failed: {err}", self.pane_id))
        })?;

        Ok(())
    }

    fn resize(&self, size: PtySize) -> PaneResult<()> {
        if self.is_dead() {
            return Err(PaneError::PaneDead);
        }

        // 原子更新尺寸
        self.rows.store(size.rows, Ordering::Relaxed);
        self.cols.store(size.cols, Ordering::Relaxed);
        self.pixel_width.store(size.pixel_width, Ordering::Relaxed);
        self.pixel_height.store(size.pixel_height, Ordering::Relaxed);

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
            tracing::error!("Pane {:?} resize failed: {}", self.pane_id, err);
            PaneError::Internal(format!("Pane {:?} PTY resize failed: {err}", self.pane_id))
        })?;

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
        self.dead.store(true, Ordering::Relaxed);
    }

    fn get_size(&self) -> PtySize {
        PtySize {
            rows: self.rows.load(Ordering::Relaxed),
            cols: self.cols.load(Ordering::Relaxed),
            pixel_width: self.pixel_width.load(Ordering::Relaxed),
            pixel_height: self.pixel_height.load(Ordering::Relaxed),
        }
    }

    fn shell_name(&self) -> &str {
        &self.shell_name
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
