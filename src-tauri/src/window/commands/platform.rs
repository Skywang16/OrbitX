/*!
 * 平台信息相关命令
 *
 * 负责获取操作系统、架构等平台相关信息
 */

use super::*;
use crate::api_success;
use crate::utils::TauriApiResult;

/// 获取平台信息
///
/// - 缓存机制：首次检测后缓存结果，后续直接返回缓存
#[tauri::command]
pub async fn get_platform_info(state: State<'_, WindowState>) -> TauriApiResult<PlatformInfo> {
    debug!("开始获取平台信息");

    // 尝试从配置管理器获取缓存的平台信息
    let platform_info = state
        .with_config_manager(|config| Ok(config.get_platform_info().cloned()))
        .await
        .to_tauri()?;

    // 如果有缓存的平台信息，直接返回
    if let Some(info) = platform_info {
        debug!(
            "从缓存获取平台信息: platform={}, arch={}, is_mac={}",
            info.platform, info.arch, info.is_mac
        );
        return Ok(api_success!(info));
    }

    debug!("首次检测平台信息");

    // 首次检测平台信息
    let platform_info = PlatformInfo {
        platform: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        os_version: detect_os_version(),
        is_mac: cfg!(target_os = "macos"),
    };

    // 缓存平台信息到配置管理器
    state
        .with_config_manager_mut(|config| {
            config.set_platform_info(platform_info.clone());
            Ok(())
        })
        .await
        .to_tauri()?;

    debug!(
        "平台信息检测完成并已缓存: platform={}, arch={}, os_version={}, is_mac={}",
        platform_info.platform, platform_info.arch, platform_info.os_version, platform_info.is_mac
    );

    Ok(api_success!(platform_info))
}

/// 检测操作系统版本
fn detect_os_version() -> String {
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output()
        {
            if let Ok(version) = String::from_utf8(output.stdout) {
                return version.trim().to_string();
            }
        }
        "macOS Unknown".to_string()
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(contents) = std::fs::read_to_string("/etc/os-release") {
            for line in contents.lines() {
                if line.starts_with("PRETTY_NAME=") {
                    let version = line.trim_start_matches("PRETTY_NAME=").trim_matches('"');
                    return version.to_string();
                }
            }
        }
        "Linux Unknown".to_string()
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = std::process::Command::new("cmd")
            .args(&["/C", "ver"])
            .output()
        {
            if let Ok(version) = String::from_utf8(output.stdout) {
                return version.trim().to_string();
            }
        }
        "Windows Unknown".to_string()
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        "Unknown OS".to_string()
    }
}
