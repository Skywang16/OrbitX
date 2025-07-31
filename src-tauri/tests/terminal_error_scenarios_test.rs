/*!
 * 终端操作错误场景测试
 *
 * 测试终端创建、写入、关闭等操作的异常情况和错误处理
 */

use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

use terminal_lib::mux::{MuxError, PaneError, PaneId, PtySize, TerminalConfig, TerminalMux};
use terminal_lib::utils::error::AppError;

/// 创建测试用的终端配置
fn create_test_config() -> TerminalConfig {
    TerminalConfig::default()
}

/// 创建无效的终端配置（用于测试错误场景）
fn create_invalid_config() -> TerminalConfig {
    let mut config = TerminalConfig::default();
    config.shell_config.program = "/nonexistent/shell".to_string();
    config
}

/// 创建测试用的PTY大小
fn create_test_size() -> PtySize {
    PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 640,
        pixel_height: 480,
    }
}

#[cfg(test)]
mod pane_creation_error_tests {
    use super::*;

    #[tokio::test]
    async fn test_create_pane_with_invalid_shell() {
        let mux = TerminalMux::new();
        let invalid_config = create_invalid_config();
        let size = create_test_size();

        let result = mux.create_pane_with_config(size, &invalid_config).await;

        // 应该失败，因为shell路径不存在
        assert!(result.is_err());

        if let Err(error) = result {
            match error {
                MuxError::PtyError(msg) => {
                    assert!(msg.contains("PTY创建错误") || msg.contains("进程启动错误"));
                }
                _ => panic!("期望PTY错误，但得到: {:?}", error),
            }
        }
    }

    #[tokio::test]
    async fn test_create_pane_with_zero_size() {
        let mux = TerminalMux::new();
        let config = create_test_config();
        let zero_size = PtySize {
            rows: 0,
            cols: 0,
            pixel_width: 0,
            pixel_height: 0,
        };

        let result = mux.create_pane_with_config(zero_size, &config).await;

        // 某些系统可能允许零大小，某些可能不允许
        // 我们主要测试错误处理是否正确
        if result.is_err() {
            assert!(matches!(result.unwrap_err(), MuxError::PtyError(_)));
        }
    }

    #[tokio::test]
    async fn test_create_pane_with_extreme_size() {
        let mux = TerminalMux::new();
        let config = create_test_config();
        let extreme_size = PtySize {
            rows: u16::MAX,
            cols: u16::MAX,
            pixel_width: u16::MAX,
            pixel_height: u16::MAX,
        };

        let result = mux.create_pane_with_config(extreme_size, &config).await;

        // 极端大小可能导致创建失败
        if result.is_err() {
            assert!(matches!(result.unwrap_err(), MuxError::PtyError(_)));
        }
    }

    #[tokio::test]
    async fn test_concurrent_pane_creation_stress() {
        let mux = Arc::new(TerminalMux::new());
        let config = create_test_config();
        let size = create_test_size();

        // 并发创建大量面板来测试资源限制
        let mut handles = vec![];

        for i in 0..50 {
            let mux_clone = mux.clone();
            let config_clone = config.clone();
            let size_clone = size;

            let handle = tokio::spawn(async move {
                let result = mux_clone
                    .create_pane_with_config(size_clone, &config_clone)
                    .await;
                (i, result)
            });
            handles.push(handle);
        }

        let mut success_count = 0;
        let mut error_count = 0;
        let mut created_panes = vec![];

        for handle in handles {
            let (i, result) = handle.await.unwrap();
            match result {
                Ok(pane_id) => {
                    success_count += 1;
                    created_panes.push(pane_id);
                }
                Err(e) => {
                    error_count += 1;
                    println!("面板 {} 创建失败: {:?}", i, e);
                }
            }
        }

        println!("成功创建: {}, 失败: {}", success_count, error_count);

        // 清理创建的面板
        for pane_id in created_panes {
            let _ = mux.remove_pane(pane_id);
        }

        // 至少应该有一些成功的创建
        assert!(success_count > 0);
    }
}

#[cfg(test)]
mod pane_write_error_tests {
    use super::*;

    #[tokio::test]
    async fn test_write_to_nonexistent_pane() {
        let mux = TerminalMux::new();
        let nonexistent_pane_id = PaneId::new(999999);
        let data = b"test data";

        let result = mux.write_to_pane(nonexistent_pane_id, data);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MuxError::PaneNotFound(_)));
    }

    #[tokio::test]
    async fn test_write_to_closed_pane() {
        let mux = TerminalMux::new();
        let config = create_test_config();
        let size = create_test_size();

        // 创建面板
        let pane_id = mux.create_pane_with_config(size, &config).await.unwrap();

        // 关闭面板
        mux.remove_pane(pane_id).unwrap();

        // 尝试写入已关闭的面板
        let data = b"test data";
        let result = mux.write_to_pane(pane_id, data);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MuxError::PaneNotFound(_)));
    }

    #[tokio::test]
    async fn test_write_large_data() {
        let mux = TerminalMux::new();
        let config = create_test_config();
        let size = create_test_size();

        let pane_id = mux.create_pane_with_config(size, &config).await.unwrap();

        // 创建大量数据（1MB）
        let large_data = vec![b'A'; 1024 * 1024];

        let result = mux.write_to_pane(pane_id, &large_data);

        // 大数据写入可能成功也可能失败，取决于系统缓冲区
        match result {
            Ok(_) => {
                // 写入成功，验证面板仍然活跃
                assert!(mux.get_pane(pane_id).is_some());
            }
            Err(e) => {
                // 写入失败，应该是合理的错误
                assert!(matches!(e, MuxError::PtyError(_) | MuxError::IoError(_)));
            }
        }

        // 清理
        let _ = mux.remove_pane(pane_id);
    }

    #[tokio::test]
    async fn test_concurrent_writes_to_same_pane() {
        let mux = Arc::new(TerminalMux::new());
        let config = create_test_config();
        let size = create_test_size();

        let pane_id = mux.create_pane_with_config(size, &config).await.unwrap();

        // 并发写入同一个面板
        let mut handles = vec![];

        for i in 0..20 {
            let mux_clone = mux.clone();
            let data = format!("data from thread {}\n", i).into_bytes();

            let handle = tokio::spawn(async move { mux_clone.write_to_pane(pane_id, &data) });
            handles.push(handle);
        }

        let mut success_count = 0;
        let mut error_count = 0;

        for handle in handles {
            match handle.await.unwrap() {
                Ok(_) => success_count += 1,
                Err(_) => error_count += 1,
            }
        }

        println!("并发写入 - 成功: {}, 失败: {}", success_count, error_count);

        // 大部分写入应该成功
        assert!(success_count > error_count);

        // 清理
        let _ = mux.remove_pane(pane_id);
    }
}

#[cfg(test)]
mod pane_lifecycle_error_tests {
    use super::*;

    #[tokio::test]
    async fn test_remove_nonexistent_pane() {
        let mux = TerminalMux::new();
        let nonexistent_pane_id = PaneId::new(999999);

        let result = mux.remove_pane(nonexistent_pane_id);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MuxError::PaneNotFound(_)));
    }

    #[tokio::test]
    async fn test_double_remove_pane() {
        let mux = TerminalMux::new();
        let config = create_test_config();
        let size = create_test_size();

        let pane_id = mux.create_pane_with_config(size, &config).await.unwrap();

        // 第一次移除应该成功
        let result1 = mux.remove_pane(pane_id);
        assert!(result1.is_ok());

        // 第二次移除应该失败
        let result2 = mux.remove_pane(pane_id);
        assert!(result2.is_err());
        assert!(matches!(result2.unwrap_err(), MuxError::PaneNotFound(_)));
    }

    #[tokio::test]
    async fn test_resize_nonexistent_pane() {
        let mux = TerminalMux::new();
        let nonexistent_pane_id = PaneId::new(999999);
        let new_size = create_test_size();

        let result = mux.resize_pane(nonexistent_pane_id, new_size);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MuxError::PaneNotFound(_)));
    }

    #[tokio::test]
    async fn test_resize_with_invalid_size() {
        let mux = TerminalMux::new();
        let config = create_test_config();
        let size = create_test_size();

        let pane_id = mux.create_pane_with_config(size, &config).await.unwrap();

        // 尝试调整为零大小
        let zero_size = PtySize {
            rows: 0,
            cols: 0,
            pixel_width: 0,
            pixel_height: 0,
        };

        let result = mux.resize_pane(pane_id, zero_size);

        // 某些系统可能允许零大小，某些可能不允许
        if result.is_err() {
            assert!(matches!(result.unwrap_err(), MuxError::PtyError(_)));
        }

        // 清理
        let _ = mux.remove_pane(pane_id);
    }

    #[tokio::test]
    async fn test_get_nonexistent_pane() {
        let mux = TerminalMux::new();
        let nonexistent_pane_id = PaneId::new(999999);

        let pane_option = mux.get_pane(nonexistent_pane_id);

        // 应该返回None，因为面板不存在
        assert!(pane_option.is_none());
    }
}

#[cfg(test)]
mod io_error_handling_tests {
    use super::*;

    #[tokio::test]
    async fn test_io_timeout_scenarios() {
        let mux = TerminalMux::new();
        let config = create_test_config();
        let size = create_test_size();

        let pane_id = mux.create_pane_with_config(size, &config).await.unwrap();

        // 测试超时场景 - 尝试在短时间内获取输出
        let timeout_result = timeout(Duration::from_millis(100), async {
            // 连续尝试获取输出，可能触发超时
            for _ in 0..1000 {
                let _ = mux.get_pane(pane_id);
                tokio::task::yield_now().await;
            }
        })
        .await;

        // 超时是预期的行为
        if timeout_result.is_err() {
            println!("操作按预期超时");
        }

        // 清理
        let _ = mux.remove_pane(pane_id);
    }

    #[tokio::test]
    async fn test_resource_exhaustion_simulation() {
        let mux = TerminalMux::new();
        let config = create_test_config();
        let size = create_test_size();

        // 尝试创建大量面板来模拟资源耗尽
        let mut created_panes = vec![];
        let mut creation_errors = vec![];

        for i in 0..100 {
            match mux.create_pane_with_config(size, &config).await {
                Ok(pane_id) => {
                    created_panes.push(pane_id);
                }
                Err(e) => {
                    creation_errors.push((i, e));
                    break; // 停止创建更多面板
                }
            }
        }

        println!("成功创建 {} 个面板", created_panes.len());
        println!("遇到 {} 个创建错误", creation_errors.len());

        // 验证错误类型
        for (i, error) in &creation_errors {
            println!("面板 {} 创建失败: {:?}", i, error);
            assert!(matches!(
                error,
                MuxError::PtyError(_) | MuxError::ResourceExhausted(_) | MuxError::Internal(_)
            ));
        }

        // 清理所有创建的面板
        for pane_id in created_panes {
            let _ = mux.remove_pane(pane_id);
        }
    }

    #[tokio::test]
    async fn test_rapid_create_destroy_cycle() {
        let mux = TerminalMux::new();
        let config = create_test_config();
        let size = create_test_size();

        // 快速创建和销毁面板的循环
        for i in 0..20 {
            match mux.create_pane_with_config(size, &config).await {
                Ok(pane_id) => {
                    // 立即写入一些数据
                    let data = format!("test data {}\n", i).into_bytes();
                    let _ = mux.write_to_pane(pane_id, &data);

                    // 立即销毁
                    match mux.remove_pane(pane_id) {
                        Ok(_) => {}
                        Err(e) => {
                            println!("销毁面板 {} 失败: {:?}", i, e);
                        }
                    }
                }
                Err(e) => {
                    println!("创建面板 {} 失败: {:?}", i, e);
                }
            }
        }
    }
}

#[cfg(test)]
mod error_conversion_tests {
    use super::*;

    #[test]
    fn test_pane_error_to_mux_error_conversion() {
        let pane_errors = vec![
            PaneError::PaneClosed,
            PaneError::WriteError("写入失败".to_string()),
            PaneError::ReadError("读取失败".to_string()),
            PaneError::ResizeError("调整大小失败".to_string()),
            PaneError::PtyCreationError("PTY创建失败".to_string()),
            PaneError::ProcessStartError("进程启动失败".to_string()),
        ];

        for pane_error in pane_errors {
            let mux_error: MuxError = pane_error.into();

            match mux_error {
                MuxError::Internal(msg) => {
                    assert!(msg.contains("面板已关闭"));
                }
                MuxError::PtyError(msg) => {
                    assert!(msg.contains("错误"));
                }
                _ => panic!("意外的错误转换: {:?}", mux_error),
            }
        }
    }

    #[test]
    fn test_mux_error_to_app_error_conversion() {
        let mux_errors = vec![
            MuxError::PaneNotFound(PaneId::new(123)),
            MuxError::PaneAlreadyExists(PaneId::new(456)),
            MuxError::PtyError("PTY错误".to_string()),
            MuxError::Internal("内部错误".to_string()),
            MuxError::ConfigError("配置错误".to_string()),
        ];

        for mux_error in mux_errors {
            let app_error: AppError = mux_error.into();

            match app_error {
                AppError::Mux {
                    message,
                    pane_id: _,
                    error_code,
                } => {
                    assert!(!message.is_empty());
                    assert!(error_code.is_some());
                    // pane_id 可能存在也可能不存在，取决于错误类型
                }
                _ => panic!("意外的错误转换: {:?}", app_error),
            }
        }
    }

    #[test]
    fn test_io_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "权限被拒绝");

        let mux_error: MuxError = io_error.into();

        match mux_error {
            MuxError::IoError(e) => {
                assert_eq!(e.kind(), std::io::ErrorKind::PermissionDenied);
            }
            _ => panic!("意外的错误转换: {:?}", mux_error),
        }
    }
}

#[cfg(test)]
mod error_recovery_tests {
    use super::*;

    #[tokio::test]
    async fn test_recovery_after_pane_creation_failure() {
        let mux = TerminalMux::new();
        let invalid_config = create_invalid_config();
        let valid_config = create_test_config();
        let size = create_test_size();

        // 尝试使用无效配置创建面板（应该失败）
        let result1 = mux.create_pane_with_config(size, &invalid_config).await;
        assert!(result1.is_err());

        // 使用有效配置创建面板（应该成功）
        let result2 = mux.create_pane_with_config(size, &valid_config).await;
        assert!(result2.is_ok());

        // 清理
        if let Ok(pane_id) = result2 {
            let _ = mux.remove_pane(pane_id);
        }
    }

    #[tokio::test]
    async fn test_mux_state_consistency_after_errors() {
        let mux = TerminalMux::new();
        let config = create_test_config();
        let size = create_test_size();

        // 创建一些面板
        let pane1 = mux.create_pane_with_config(size, &config).await.unwrap();
        let pane2 = mux.create_pane_with_config(size, &config).await.unwrap();

        let initial_count = mux.pane_count();
        assert_eq!(initial_count, 2);

        // 尝试一些会失败的操作
        let _ = mux.write_to_pane(PaneId::new(999999), b"test");
        let _ = mux.remove_pane(PaneId::new(999999));
        let _ = mux.resize_pane(PaneId::new(999999), size);

        // 验证面板计数没有改变
        assert_eq!(mux.pane_count(), initial_count);

        // 验证现有面板仍然可用
        assert!(mux.get_pane(pane1).is_some());
        assert!(mux.get_pane(pane2).is_some());

        // 清理
        let _ = mux.remove_pane(pane1);
        let _ = mux.remove_pane(pane2);

        assert_eq!(mux.pane_count(), 0);
    }

    #[tokio::test]
    async fn test_graceful_shutdown_with_errors() {
        let mux = TerminalMux::new();
        let config = create_test_config();
        let size = create_test_size();

        // 创建一些面板
        let _pane1 = mux.create_pane_with_config(size, &config).await.unwrap();
        let _pane2 = mux.create_pane_with_config(size, &config).await.unwrap();

        // 关闭mux（应该清理所有资源）
        let shutdown_result = mux.shutdown();

        // 关闭操作应该成功，即使有一些内部错误
        match shutdown_result {
            Ok(_) => {
                // 验证所有面板都已清理
                assert_eq!(mux.pane_count(), 0);
            }
            Err(e) => {
                println!("关闭时出现错误（可能是预期的）: {:?}", e);
                // 即使关闭时有错误，面板计数也应该为0
                assert_eq!(mux.pane_count(), 0);
            }
        }
    }
}
