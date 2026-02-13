use super::types::NodeVersionManager;
use std::env;
use std::path::PathBuf;
use std::process::Command;

// 检测当前系统使用的 Node 版本管理器
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

fn check_volta() -> bool {
    if let Ok(volta_home) = env::var("VOLTA_HOME") {
        let volta_path = PathBuf::from(volta_home);
        if volta_path.exists() {
            return true;
        }
    }

    if let Ok(home) = env::var("HOME") {
        let volta_path = PathBuf::from(home).join(".volta");
        if volta_path.exists() {
            return true;
        }
    }

    false
}

fn check_fnm() -> bool {
    if let Ok(fnm_dir) = env::var("FNM_DIR") {
        let fnm_path = PathBuf::from(fnm_dir);
        if fnm_path.exists() {
            return true;
        }
    }

    if let Ok(home) = env::var("HOME") {
        let fnm_path = PathBuf::from(home).join(".local/share/fnm");
        if fnm_path.exists() {
            return true;
        }
    }

    if let Ok(output) = Command::new("fnm").arg("--version").output() {
        return output.status.success();
    }

    false
}

fn check_nvm() -> bool {
    if let Ok(nvm_dir) = env::var("NVM_DIR") {
        let nvm_path = PathBuf::from(nvm_dir);
        if nvm_path.exists() {
            return true;
        }
    }

    if let Ok(home) = env::var("HOME") {
        let nvm_path = PathBuf::from(home).join(".nvm");
        if nvm_path.exists() {
            return true;
        }
    }

    false
}

fn check_n() -> bool {
    if let Ok(output) = Command::new("n").arg("--version").output() {
        return output.status.success();
    }
    false
}

fn check_asdf() -> bool {
    if let Ok(output) = Command::new("asdf").arg("plugin").arg("list").output() {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return stdout.contains("nodejs");
        }
    }
    false
}

// 获取当前激活的 Node 版本
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

        let command = format!("cd '{}' && node -v", dir);
        // 在非 Windows 平台以 login shell 执行，Windows 下不传递无效的 "-l" 参数
        let output_result = {
            let mut cmd = Command::new(shell_cmd);
            if !cfg!(target_os = "windows") {
                cmd.arg("-l");
            }
            cmd.arg(shell_arg).arg(&command).output()
        };
        if let Ok(output) = output_result {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !version.is_empty() {
                    let cleaned = version.trim_start_matches('v');
                    return Ok(Some(format!("v{}", cleaned)));
                }
            }
        }
    }

    if let Ok(output) = Command::new("which").arg("node").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout);
            if let Some(version) = parse_node_path(&path) {
                return Ok(Some(version));
            }
        }
    }

    if let Ok(output) = Command::new("node").arg("--version").output() {
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let cleaned = version.trim_start_matches('v');
            return Ok(Some(format!("v{}", cleaned)));
        }
    }

    Ok(None)
}

// 从 node 路径中提取版本号
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

// 获取已安装的 Node 版本列表
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

// 从目录读取版本列表
fn read_versions_from_dir(path: PathBuf, add_v_prefix: bool) -> Result<Vec<String>, String> {
    if !path.exists() {
        return Ok(vec![]);
    }

    let entries = std::fs::read_dir(&path)
        .map_err(|e| format!("Failed to read versions directory: {}", e))?;

    let mut versions = Vec::new();
    for entry in entries.flatten() {
        if let Some(name) = entry.file_name().to_str() {
            if !name.starts_with('.') && entry.path().is_dir() {
                let version = if add_v_prefix && !name.starts_with('v') {
                    format!("v{}", name)
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

// 获取 nvm 版本列表
fn get_nvm_versions() -> Result<Vec<String>, String> {
    let nvm_dir = env::var("NVM_DIR")
        .or_else(|_| env::var("HOME").map(|h| format!("{}/.nvm", h)))
        .map_err(|_| "Cannot determine NVM_DIR".to_string())?;

    let versions_path = PathBuf::from(nvm_dir).join("versions/node");
    // nvm 的目录通常已带 v 前缀，但为统一前端展示，这里启用 v 前缀规范化
    read_versions_from_dir(versions_path, true)
}

// 语义化版本比较
fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let a_nums: Vec<u32> = a
        .trim_start_matches('v')
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();
    let b_nums: Vec<u32> = b
        .trim_start_matches('v')
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();

    for i in 0..a_nums.len().max(b_nums.len()) {
        let a_num = a_nums.get(i).unwrap_or(&0);
        let b_num = b_nums.get(i).unwrap_or(&0);
        match a_num.cmp(b_num) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }

    std::cmp::Ordering::Equal
}

// 获取 fnm 版本列表
fn get_fnm_versions() -> Result<Vec<String>, String> {
    let fnm_dir = env::var("FNM_DIR")
        .or_else(|_| env::var("HOME").map(|h| format!("{}/.local/share/fnm", h)))
        .map_err(|_| "Cannot determine FNM_DIR".to_string())?;

    let versions_path = PathBuf::from(fnm_dir).join("node-versions");
    // 统一使用带 v 的版本字符串
    read_versions_from_dir(versions_path, true)
}

// 获取 volta 版本列表
fn get_volta_versions() -> Result<Vec<String>, String> {
    let volta_home = env::var("VOLTA_HOME")
        .or_else(|_| env::var("HOME").map(|h| format!("{}/.volta", h)))
        .map_err(|_| "Cannot determine VOLTA_HOME".to_string())?;

    let inventory_path = PathBuf::from(volta_home).join("tools/inventory/node");
    read_versions_from_dir(inventory_path, true)
}

// 获取 n 版本列表
fn get_n_versions() -> Result<Vec<String>, String> {
    let n_prefix = env::var("N_PREFIX").unwrap_or_else(|_| "/usr/local".to_string());

    let versions_path = PathBuf::from(n_prefix).join("n/versions/node");
    read_versions_from_dir(versions_path, true)
}

// 获取 asdf 版本列表
fn get_asdf_versions() -> Result<Vec<String>, String> {
    let asdf_dir = env::var("ASDF_DATA_DIR")
        .or_else(|_| env::var("HOME").map(|h| format!("{}/.asdf", h)))
        .map_err(|_| "Cannot determine ASDF_DATA_DIR".to_string())?;

    let versions_path = PathBuf::from(asdf_dir).join("installs/nodejs");
    read_versions_from_dir(versions_path, true)
}
