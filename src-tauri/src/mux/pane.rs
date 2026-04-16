//! Panel interface and implementation

use std::io::{Read, Write};
use std::process;
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::Mutex;

use crate::mux::error::{PaneError, PaneResult};
use crate::mux::shell_manager::ShellInfo;
use crate::mux::{MuxSessionConfig, PaneId, PtySize};
use portable_pty::{CommandBuilder, MasterPty, PtySize as PortablePtySize, SlavePty};

pub trait Pane: Send + Sync {
    fn pane_id(&self) -> PaneId;

    fn write(&self, data: &[u8]) -> PaneResult<()>;

    fn resize(&self, size: PtySize) -> PaneResult<()>;

    fn reader(&self) -> PaneResult<Box<dyn Read + Send>>;

    fn is_dead(&self) -> bool;

    fn mark_dead(&self);

    fn get_size(&self) -> PtySize;

    /// Get the complete shell information used at creation
    fn shell_info(&self) -> &ShellInfo;
}

pub struct LocalPane {
    pane_id: PaneId,
    rows: AtomicU16,
    cols: AtomicU16,
    pixel_width: AtomicU16,
    pixel_height: AtomicU16,
    dead: AtomicBool,
    shell_info: ShellInfo,
    master: Mutex<Box<dyn MasterPty + Send>>,
    writer: Mutex<Box<dyn Write + Send>>,
    _slave: Mutex<Box<dyn SlavePty + Send>>,
}

struct SpawnedProcess {
    master: Mutex<Box<dyn MasterPty + Send>>,
    writer: Mutex<Box<dyn Write + Send>>,
    slave: Mutex<Box<dyn SlavePty + Send>>,
}

impl LocalPane {
    pub fn new(pane_id: PaneId, size: PtySize) -> PaneResult<Self> {
        let config = MuxSessionConfig::with_default_shell().map_err(PaneError::Internal)?;
        Self::new_with_config(pane_id, size, &config)
    }

    /// Create a new local panel
    pub fn new_with_config(
        pane_id: PaneId,
        size: PtySize,
        config: &MuxSessionConfig,
    ) -> PaneResult<Self> {
        let pty_pair = Self::create_pty(pane_id, size)?;
        let mut cmd = Self::build_command(config)?;
        Self::setup_shell_integration(&mut cmd, config)?;
        let spawned = Self::spawn_process(pane_id, pty_pair, cmd)?;

        Ok(Self {
            pane_id,
            rows: AtomicU16::new(size.rows),
            cols: AtomicU16::new(size.cols),
            pixel_width: AtomicU16::new(size.pixel_width),
            pixel_height: AtomicU16::new(size.pixel_height),
            dead: AtomicBool::new(false),
            shell_info: config.shell_config.shell_info.clone(),
            master: spawned.master,
            writer: spawned.writer,
            _slave: spawned.slave,
        })
    }

    /// Create PTY
    fn create_pty(pane_id: PaneId, size: PtySize) -> PaneResult<portable_pty::PtyPair> {
        let pty_system = portable_pty::native_pty_system();

        let pty_size = PortablePtySize {
            rows: size.rows,
            cols: size.cols,
            pixel_width: size.pixel_width,
            pixel_height: size.pixel_height,
        };

        pty_system.openpty(pty_size).map_err(|err| {
            PaneError::Internal(format!("Failed to create PTY for {pane_id:?}: {err}"))
        })
    }

    /// Build shell command
    fn build_command(config: &MuxSessionConfig) -> PaneResult<CommandBuilder> {
        let mut cmd = CommandBuilder::new(&config.shell_config.shell_info.path);
        cmd.args(&config.shell_config.args);

        if let Some(cwd) = &config.shell_config.working_directory {
            cmd.cwd(cwd);
        }

        if let Some(env_vars) = &config.shell_config.env {
            for (key, value) in env_vars {
                cmd.env(key, value);
            }
        }

        // Add basic environment variables
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        cmd.env("TERM_PROGRAM", "OrbitX");
        cmd.env("LANG", "en_US.UTF-8");
        cmd.env("LC_ALL", "en_US.UTF-8");
        cmd.env("LC_CTYPE", "en_US.UTF-8");

        Ok(cmd)
    }

    /// Setup Shell Integration
    fn setup_shell_integration(
        cmd: &mut CommandBuilder,
        config: &MuxSessionConfig,
    ) -> PaneResult<()> {
        let shell_type =
            crate::shell::ShellType::from_program(&config.shell_config.shell_info.path);

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

    /// Setup Zsh Shell Integration
    fn setup_zsh_integration(cmd: &mut CommandBuilder, integration_script: &str) -> PaneResult<()> {
        let temp_dir = std::env::temp_dir().join(format!("orbitx-{}", process::id()));

        std::fs::create_dir_all(&temp_dir).map_err(|e| {
            PaneError::Internal(format!("Failed to create temporary directory: {e}"))
        })?;

        let temp_zshrc = temp_dir.join(".zshrc");
        let mut file = std::fs::File::create(&temp_zshrc)
            .map_err(|e| PaneError::Internal(format!("Failed to create .zshrc: {e}")))?;

        write_shell_init_line(&mut file, "# Load user zshrc FIRST (so nvm etc. loads)")?;
        write_shell_init_line(&mut file, "[[ -f ~/.zshrc ]] && source ~/.zshrc")?;
        write_shell_init_line(&mut file, "")?;
        write_shell_init_line(
            &mut file,
            "# OrbitX Shell Integration (after user env loaded)",
        )?;
        write_shell_init_line(&mut file, integration_script)?;

        if let Some(original_zdotdir) = std::env::var_os("ZDOTDIR") {
            cmd.env("ORBITX_ORIGINAL_ZDOTDIR", original_zdotdir);
        }
        cmd.env("ZDOTDIR", &temp_dir);

        Ok(())
    }

    /// Setup Bash Shell Integration
    fn setup_bash_integration(
        cmd: &mut CommandBuilder,
        integration_script: &str,
    ) -> PaneResult<()> {
        let temp_dir = std::env::temp_dir().join(format!("orbitx-{}", process::id()));

        std::fs::create_dir_all(&temp_dir).map_err(|e| {
            PaneError::Internal(format!("Failed to create temporary directory: {e}"))
        })?;

        let temp_bashrc = temp_dir.join(".bashrc");
        let mut file = std::fs::File::create(&temp_bashrc)
            .map_err(|e| PaneError::Internal(format!("Failed to create .bashrc: {e}")))?;

        write_shell_init_line(&mut file, "# Load user bashrc FIRST (so nvm etc. loads)")?;
        write_shell_init_line(&mut file, "[[ -f ~/.bashrc ]] && source ~/.bashrc")?;
        write_shell_init_line(&mut file, "")?;
        write_shell_init_line(
            &mut file,
            "# OrbitX Shell Integration (after user env loaded)",
        )?;
        write_shell_init_line(&mut file, integration_script)?;

        cmd.env("BASH_ENV", temp_bashrc);

        Ok(())
    }

    /// Spawn child process
    fn spawn_process(
        pane_id: PaneId,
        pty_pair: portable_pty::PtyPair,
        cmd: CommandBuilder,
    ) -> PaneResult<SpawnedProcess> {
        pty_pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| PaneError::Spawn {
                reason: e.to_string(),
            })?;

        let writer = pty_pair.master.take_writer().map_err(|err| {
            PaneError::Internal(format!(
                "Failed to acquire PTY writer for {pane_id:?}: {err}"
            ))
        })?;

        let master = Mutex::new(pty_pair.master);
        let writer = Mutex::new(writer);
        let slave = Mutex::new(pty_pair.slave);

        Ok(SpawnedProcess {
            master,
            writer,
            slave,
        })
    }
}

fn write_shell_init_line(file: &mut std::fs::File, line: &str) -> PaneResult<()> {
    use std::io::Write;

    writeln!(file, "{line}")
        .map_err(|err| PaneError::Internal(format!("Failed to write shell init file: {err}")))?;
    Ok(())
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

        // Write data using Write trait
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

        // Atomically update size
        self.rows.store(size.rows, Ordering::Relaxed);
        self.cols.store(size.cols, Ordering::Relaxed);
        self.pixel_width.store(size.pixel_width, Ordering::Relaxed);
        self.pixel_height
            .store(size.pixel_height, Ordering::Relaxed);

        // Resize PTY
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

    fn shell_info(&self) -> &ShellInfo {
        &self.shell_info
    }
}

// Convenience method implementations
impl LocalPane {
    /// Write string data
    pub fn write_str(&self, data: &str) -> PaneResult<()> {
        self.write(data.as_bytes())
    }

    /// Write string with newline
    pub fn write_line(&self, data: &str) -> PaneResult<()> {
        let mut line = data.to_string();
        line.push('\n');
        self.write(line.as_bytes())
    }
}
