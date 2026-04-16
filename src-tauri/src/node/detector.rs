use super::types::NodeVersionManager;
use std::env;
use std::path::PathBuf;
use std::process::Command;
use tracing::warn;

// Detect the Node version manager used by the current system
pub fn detect_version_manager() -> NodeVersionManager {
    if check_volta() {
        return NodeVersionManager::Volta;
    }
    if check_fnm() {
        return NodeVersionManager::Fnm;
    }
    if check_nvm() {
        return NodeVersionManager::Nvm;
    }
    if check_n() {
        return NodeVersionManager::N;
    }
    if check_asdf() {
        return NodeVersionManager::Asdf;
    }
    NodeVersionManager::Unknown
}

fn home_dir() -> Option<PathBuf> {
    #[allow(deprecated)]
    std::env::home_dir()
}

fn check_volta() -> bool {
    env::var("VOLTA_HOME")
        .ok()
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .is_some()
        || home_dir().is_some_and(|h| h.join(".volta").exists())
}

fn check_fnm() -> bool {
    env::var("FNM_DIR")
        .ok()
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .is_some()
        || home_dir().is_some_and(|h| h.join(".local/share/fnm").exists())
        || matches!(Command::new("fnm").arg("--version").output(), Ok(o) if o.status.success())
}

fn check_nvm() -> bool {
    env::var("NVM_DIR")
        .ok()
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .is_some()
        || home_dir().is_some_and(|h| h.join(".nvm").exists())
}

fn check_n() -> bool {
    matches!(Command::new("n").arg("--version").output(), Ok(o) if o.status.success())
}

fn check_asdf() -> bool {
    matches!(
        Command::new("asdf").arg("plugin").arg("list").output(),
        Ok(o) if o.status.success() && String::from_utf8_lossy(&o.stdout).contains("nodejs")
    )
}

// Get currently active Node version
pub fn get_current_version(cwd: Option<&str>) -> Result<Option<String>, String> {
    if let Some(dir) = cwd {
        let shell_cmd = if cfg!(target_os = "windows") {
            "cmd"
        } else {
            "zsh"
        };

        let shell_arg = if cfg!(target_os = "windows") {
            "/C"
        } else {
            "-c"
        };

        let command = format!("cd '{dir}' && node -v");
        // Execute as login shell on non-Windows platforms; do not pass invalid "-l" parameter on Windows
        let output_result = {
            let mut cmd = Command::new(shell_cmd);
            if !cfg!(target_os = "windows") {
                cmd.arg("-l");
            }
            cmd.arg(shell_arg).arg(&command).output()
        };
        match output_result {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !version.is_empty() {
                    let cleaned = version.trim_start_matches('v');
                    return Ok(Some(format!("v{cleaned}")));
                }
            }
            Ok(output) => warn!(
                "failed to get node version from cwd '{}': command exited with status {}",
                dir, output.status
            ),
            Err(err) => warn!("failed to get node version from cwd '{}': {}", dir, err),
        }
    }

    match Command::new("which").arg("node").output() {
        Ok(output) if output.status.success() => {
            let path = String::from_utf8_lossy(&output.stdout);
            if let Some(version) = parse_node_path(&path) {
                return Ok(Some(version));
            }
        }
        Ok(_) => {}
        Err(err) => warn!("failed to execute which node: {}", err),
    }

    match Command::new("node").arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let cleaned = version.trim_start_matches('v');
            return Ok(Some(format!("v{cleaned}")));
        }
        Ok(_) => {}
        Err(err) => warn!("failed to execute node --version: {}", err),
    }

    Ok(None)
}

// Extract version number from node path
fn parse_node_path(path: &str) -> Option<String> {
    use once_cell::sync::Lazy;
    use regex::Regex;

    // Compile regex once
    static NODE_VERSION_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"/(?:node|node-versions|nodejs)/v?(\d+\.\d+\.\d+)").unwrap());

    NODE_VERSION_RE
        .captures(path)
        .and_then(|c| c.get(1))
        .map(|m| format!("v{}", m.as_str()))
}

// Get list of installed Node versions
pub fn get_node_versions(manager: &NodeVersionManager) -> Result<Vec<String>, String> {
    match manager {
        NodeVersionManager::Nvm => get_nvm_versions(),
        NodeVersionManager::Fnm => get_fnm_versions(),
        NodeVersionManager::Volta => get_volta_versions(),
        NodeVersionManager::N => get_n_versions(),
        NodeVersionManager::Asdf => get_asdf_versions(),
        NodeVersionManager::Unknown => Ok(vec![]),
    }
}

// Read version list from directory
fn read_versions_from_dir(path: PathBuf, add_v_prefix: bool) -> Result<Vec<String>, String> {
    if !path.exists() {
        return Ok(vec![]);
    }

    let entries =
        std::fs::read_dir(&path).map_err(|e| format!("Failed to read versions directory: {e}"))?;

    let mut versions = Vec::new();
    for entry in entries.flatten() {
        if let Some(name) = entry.file_name().to_str() {
            if !name.starts_with('.') && entry.path().is_dir() {
                let version = if add_v_prefix && !name.starts_with('v') {
                    format!("v{name}")
                } else {
                    name.to_string()
                };
                versions.push(version);
            }
        }
    }

    versions.sort_by(|a, b| compare_versions(b, a));
    Ok(versions)
}

// Get nvm version list
fn get_nvm_versions() -> Result<Vec<String>, String> {
    let nvm_dir = env_path_or_home_suffix("NVM_DIR", ".nvm")?;
    let versions_path = PathBuf::from(nvm_dir).join("versions/node");
    read_versions_from_dir(versions_path, true)
}

// Semantic version comparison
fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let a_nums = parse_semver_components(a);
    let b_nums = parse_semver_components(b);

    for i in 0..a_nums.len().max(b_nums.len()) {
        let a_num = match a_nums.get(i) {
            Some(value) => *value,
            None => 0,
        };
        let b_num = match b_nums.get(i) {
            Some(value) => *value,
            None => 0,
        };
        match a_num.cmp(&b_num) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }

    std::cmp::Ordering::Equal
}

// Get fnm version list
fn get_fnm_versions() -> Result<Vec<String>, String> {
    let fnm_dir = env_path_or_home_suffix("FNM_DIR", ".local/share/fnm")?;

    let versions_path = PathBuf::from(fnm_dir).join("node-versions");
    // Use version strings with v prefix uniformly
    read_versions_from_dir(versions_path, true)
}

// Get volta version list
fn get_volta_versions() -> Result<Vec<String>, String> {
    let volta_home = env_path_or_home_suffix("VOLTA_HOME", ".volta")?;

    let inventory_path = PathBuf::from(volta_home).join("tools/inventory/node");
    read_versions_from_dir(inventory_path, true)
}

// Get n version list
fn get_n_versions() -> Result<Vec<String>, String> {
    let n_prefix = match env::var("N_PREFIX") {
        Ok(path) => path,
        Err(env::VarError::NotPresent) => "/usr/local".to_string(),
        Err(err) => {
            warn!("failed to read N_PREFIX: {}", err);
            "/usr/local".to_string()
        }
    };

    let versions_path = PathBuf::from(n_prefix).join("n/versions/node");
    read_versions_from_dir(versions_path, true)
}

// Get asdf version list
fn get_asdf_versions() -> Result<Vec<String>, String> {
    let asdf_dir = env_path_or_home_suffix("ASDF_DATA_DIR", ".asdf")?;

    let versions_path = PathBuf::from(asdf_dir).join("installs/nodejs");
    read_versions_from_dir(versions_path, true)
}

fn env_path_or_home_suffix(var_name: &str, home_suffix: &str) -> Result<String, String> {
    if let Ok(value) = env::var(var_name) {
        return Ok(value);
    }
    home_dir()
        .map(|h| format!("{}/{home_suffix}", h.display()))
        .ok_or_else(|| format!("Cannot determine {var_name}"))
}

fn parse_semver_components(version: &str) -> Vec<u32> {
    let mut components = Vec::new();

    for segment in version.trim_start_matches('v').split('.') {
        match segment.parse::<u32>() {
            Ok(number) => components.push(number),
            Err(err) => warn!("failed to parse semver segment '{}': {}", segment, err),
        }
    }

    components
}
