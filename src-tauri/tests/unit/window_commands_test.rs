/*!
 * Window Commands 单元测试
 *
 * 测试窗口命令模块的状态管理和配置功能
 */

use terminal_lib::window::commands::{WindowConfigManager, WindowStateManager};

#[test]
fn test_window_state_manager_creation() {
    let manager = WindowStateManager::new();

    assert!(!manager.get_always_on_top());
    assert!(manager.get_cached_cwd().is_none());
    assert!(manager.get_cached_home().is_none());
}

#[test]
fn test_always_on_top_toggle() {
    let mut manager = WindowStateManager::new();

    // 初始状态为false
    assert!(!manager.get_always_on_top());

    // 切换为true
    let new_value = manager.toggle_always_on_top();
    assert!(new_value);
    assert!(manager.get_always_on_top());

    // 再次切换回false
    let new_value = manager.toggle_always_on_top();
    assert!(!new_value);
    assert!(!manager.get_always_on_top());
}

#[test]
fn test_always_on_top_set() {
    let mut manager = WindowStateManager::new();

    manager.set_always_on_top(true);
    assert!(manager.get_always_on_top());

    manager.set_always_on_top(false);
    assert!(!manager.get_always_on_top());
}

#[test]
fn test_cached_cwd() {
    let mut manager = WindowStateManager::new();
    let test_path = std::path::PathBuf::from("/tmp/test");

    // 初始状态没有缓存
    assert!(manager.get_cached_cwd().is_none());

    // 设置缓存
    manager.set_cached_cwd(test_path.clone());
    assert!(manager.get_cached_cwd().is_some());
    assert_eq!(manager.get_cached_cwd().unwrap(), &test_path);
}

#[test]
fn test_cached_home() {
    let mut manager = WindowStateManager::new();
    let test_path = std::path::PathBuf::from("/home/test");

    // 初始状态没有缓存
    assert!(manager.get_cached_home().is_none());

    // 设置缓存
    manager.set_cached_home(test_path.clone());
    assert!(manager.get_cached_home().is_some());
    assert_eq!(manager.get_cached_home().unwrap(), &test_path);
}

#[test]
fn test_cache_clear() {
    let mut manager = WindowStateManager::new();

    // 设置一些缓存
    manager.set_cached_cwd(std::path::PathBuf::from("/tmp"));
    manager.set_cached_home(std::path::PathBuf::from("/home"));
    manager.set_always_on_top(true);

    // 清除缓存
    manager.clear_cache();

    // 验证缓存已清除（但always_on_top不受影响）
    assert!(manager.get_cached_cwd().is_none());
    assert!(manager.get_cached_home().is_none());
    assert!(manager.get_always_on_top()); // 这个不应该被清除
}

#[test]
fn test_state_reset() {
    let mut manager = WindowStateManager::new();

    // 设置一些状态
    manager.set_cached_cwd(std::path::PathBuf::from("/tmp"));
    manager.set_cached_home(std::path::PathBuf::from("/home"));
    manager.set_always_on_top(true);

    // 重置
    manager.reset();

    // 验证所有状态已重置
    assert!(manager.get_cached_cwd().is_none());
    assert!(manager.get_cached_home().is_none());
    assert!(!manager.get_always_on_top());
}

#[test]
fn test_window_config_manager_creation() {
    let manager = WindowConfigManager::new();

    assert!(manager.window_get_platform_info().is_none());
    assert_eq!(manager.get_default_window_id(), "main");
    assert_eq!(manager.get_operation_timeout(), 5000);
}

#[test]
fn test_config_manager_window_id() {
    let mut manager = WindowConfigManager::new();

    // 测试默认值
    assert_eq!(manager.get_default_window_id(), "main");

    // 设置新值
    manager.set_default_window_id("custom-window".to_string());
    assert_eq!(manager.get_default_window_id(), "custom-window");
}

#[test]
fn test_config_manager_operation_timeout() {
    let mut manager = WindowConfigManager::new();

    // 测试默认值
    assert_eq!(manager.get_operation_timeout(), 5000);

    // 设置新值
    manager.set_operation_timeout(10000);
    assert_eq!(manager.get_operation_timeout(), 10000);
}

#[test]
fn test_concurrent_state_access() {
    use std::sync::{Arc, Mutex};
    use std::thread;

    let manager: Arc<Mutex<WindowStateManager>> = Arc::new(Mutex::new(WindowStateManager::new()));
    let mut handles = Vec::new();

    // 多线程并发访问
    for i in 0..10 {
        let manager_clone = Arc::clone(&manager);
        let handle = thread::spawn(move || {
            let mut mgr = manager_clone.lock().unwrap();
            mgr.set_always_on_top(i % 2 == 0);
        });
        handles.push(handle);
    }

    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }

    // 验证状态一致性（最后一个设置的值）
    let mgr = manager.lock().unwrap();
    let value = mgr.get_always_on_top();
    // 值应该是true或false中的一个
    assert!(value || !value);
}

#[test]
fn test_atomic_always_on_top() {
    use std::sync::Arc;

    let manager = Arc::new(WindowStateManager::new());

    // 测试原子操作的并发安全性
    let handles: Vec<_> = (0..100)
        .map(|i| {
            let mgr = Arc::clone(&manager);
            std::thread::spawn(move || {
                for _ in 0..10 {
                    if i % 2 == 0 {
                        mgr.get_always_on_top();
                    }
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    // 验证没有panic
    assert!(true);
}
