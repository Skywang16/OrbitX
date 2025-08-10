/*!
 * 终端多路复用器(Mux)集成测试
 *
 * 本文件测试Mux模块的核心功能，包括：
 * - 面板的创建、管理和销毁
 * - 数据写入和读取操作
 * - 面板大小调整功能
 * - 多面板并发操作
 * - 错误处理和资源清理
 */

use terminal_lib::mux::{get_mux, PtySize};

/// 测试Mux的基本生命周期操作
///
/// 验证面板的完整生命周期：创建 -> 操作 -> 调整 -> 销毁
#[tokio::test]
async fn test_mux_basic_lifecycle() {
    // === 准备阶段 ===
    let mux = get_mux();
    let initial_size = PtySize::new(24, 80);

    // === 创建面板 ===
    let pane_id = mux
        .create_pane(initial_size)
        .await
        .expect("应该能够成功创建新的终端面板");

    // 验证面板ID有效
    assert!(
        pane_id.as_u32() > 0,
        "面板ID应该是正数，实际值: {}",
        pane_id.as_u32()
    );

    // 验证面板存在且可访问
    let pane = mux.get_pane(pane_id).expect("新创建的面板应该能够被找到");

    // 验证初始大小设置正确
    let current_size = pane.get_size();
    assert_eq!(
        current_size.rows, initial_size.rows,
        "面板行数应该匹配初始设置: 期望 {}, 实际 {}",
        initial_size.rows, current_size.rows
    );
    assert_eq!(
        current_size.cols, initial_size.cols,
        "面板列数应该匹配初始设置: 期望 {}, 实际 {}",
        initial_size.cols, current_size.cols
    );

    // === 数据写入测试 ===
    let test_command = b"echo 'Hello, Terminal!'\n";
    let write_result = mux.write_to_pane(pane_id, test_command);

    assert!(
        write_result.is_ok(),
        "向面板写入数据应该成功，错误: {:?}",
        write_result.err()
    );

    // === 面板大小调整测试 ===
    let new_size = PtySize::new(30, 100);
    let resize_result = mux.resize_pane(pane_id, new_size);

    assert!(
        resize_result.is_ok(),
        "调整面板大小应该成功，错误: {:?}",
        resize_result.err()
    );

    // 验证大小调整生效
    let updated_pane = mux.get_pane(pane_id).expect("调整大小后面板应该仍然存在");
    let updated_size = updated_pane.get_size();

    assert_eq!(
        updated_size.rows, new_size.rows,
        "调整后的行数应该匹配: 期望 {}, 实际 {}",
        new_size.rows, updated_size.rows
    );
    assert_eq!(
        updated_size.cols, new_size.cols,
        "调整后的列数应该匹配: 期望 {}, 实际 {}",
        new_size.cols, updated_size.cols
    );

    // === 清理阶段 ===
    let remove_result = mux.remove_pane(pane_id);
    assert!(
        remove_result.is_ok(),
        "移除面板应该成功，错误: {:?}",
        remove_result.err()
    );

    // 验证面板已被完全移除
    let removed_pane = mux.get_pane(pane_id);
    assert!(removed_pane.is_none(), "移除后的面板不应该再能被找到");
}

/// 测试多面板并发管理
///
/// 验证Mux能够同时管理多个面板，包括：
/// - 创建多个不同配置的面板
/// - 并发操作多个面板
/// - 独立管理每个面板的状态
/// - 批量清理所有面板
#[tokio::test]
async fn test_multiple_panes_management() {
    // === 准备阶段 ===
    let mux = get_mux();
    let initial_pane_count = mux.pane_count();

    // 定义不同配置的面板
    let pane_configs = [
        ("小面板", PtySize::new(24, 80)),
        ("中面板", PtySize::new(30, 100)),
        ("大面板", PtySize::new(40, 120)),
    ];

    // === 创建多个面板 ===
    let mut created_panes = Vec::new();

    for (name, size) in &pane_configs {
        let pane_id = mux
            .create_pane(*size)
            .await
            .unwrap_or_else(|_| panic!("应该能够创建{}", name));

        created_panes.push((name, pane_id, size));

        // 验证面板创建成功
        let pane = mux
            .get_pane(pane_id)
            .unwrap_or_else(|| panic!("{}应该能够被找到", name));

        let actual_size = pane.get_size();
        assert_eq!(
            actual_size.rows, size.rows,
            "{}的行数应该正确: 期望 {}, 实际 {}",
            name, size.rows, actual_size.rows
        );
        assert_eq!(
            actual_size.cols, size.cols,
            "{}的列数应该正确: 期望 {}, 实际 {}",
            name, size.cols, actual_size.cols
        );
    }

    // 验证面板总数增加
    let current_pane_count = mux.pane_count();
    assert_eq!(
        current_pane_count,
        initial_pane_count + pane_configs.len(),
        "面板总数应该增加: 期望 {}, 实际 {}",
        initial_pane_count + pane_configs.len(),
        current_pane_count
    );

    // === 验证面板列表功能 ===
    let pane_list = mux.list_panes();
    for (name, pane_id, _) in &created_panes {
        assert!(
            pane_list.contains(pane_id),
            "面板列表应该包含{} (ID: {:?})",
            name,
            pane_id
        );
    }

    // === 并发操作测试 ===
    for (name, pane_id, _) in &created_panes {
        let test_data = format!("echo 'Testing {}'\n", name).into_bytes();
        let write_result = mux.write_to_pane(*pane_id, &test_data);

        assert!(
            write_result.is_ok(),
            "向{}写入数据应该成功，错误: {:?}",
            name,
            write_result.err()
        );
    }

    // === 清理阶段 ===
    for (name, pane_id, _) in &created_panes {
        let remove_result = mux.remove_pane(*pane_id);
        assert!(
            remove_result.is_ok(),
            "移除{}应该成功，错误: {:?}",
            name,
            remove_result.err()
        );
    }

    // === 验证清理结果 ===
    // 验证所有面板都已被移除
    for (name, pane_id, _) in &created_panes {
        let removed_pane = mux.get_pane(*pane_id);
        assert!(removed_pane.is_none(), "{}应该已被完全移除", name);
    }

    // 验证面板总数恢复到初始状态
    let final_pane_count = mux.pane_count();
    assert_eq!(
        final_pane_count, initial_pane_count,
        "面板总数应该恢复到初始状态: 期望 {}, 实际 {}",
        initial_pane_count, final_pane_count
    );
}

#[test]
fn test_error_scenarios() {
    let mux = get_mux();

    // 测试操作不存在的终端
    let nonexistent_pane = terminal_lib::mux::PaneId::from(99999);

    // 写入不存在的终端应该失败
    let result = mux.write_to_pane(nonexistent_pane, b"test");
    assert!(result.is_err());

    // 调整不存在终端的大小应该失败
    let result = mux.resize_pane(nonexistent_pane, PtySize::default());
    assert!(result.is_err());

    // 关闭不存在的终端应该失败
    let result = mux.remove_pane(nonexistent_pane);
    assert!(result.is_err());
}

#[test]
fn test_notification_system() {
    use std::sync::{Arc, Mutex};
    use terminal_lib::mux::MuxNotification;

    let mux = get_mux();
    let received_notifications = Arc::new(Mutex::new(Vec::new()));
    let received_clone = Arc::clone(&received_notifications);

    // 订阅通知
    let _subscriber_id = mux.subscribe(move |notification| {
        received_clone.lock().unwrap().push(notification.clone());
        true // 继续订阅
    });

    // 发送一些通知
    mux.notify(MuxNotification::PaneAdded(terminal_lib::mux::PaneId::from(
        1,
    )));
    mux.notify(MuxNotification::PaneOutput {
        pane_id: terminal_lib::mux::PaneId::from(1),
        data: b"test output".to_vec().into(),
    });
    mux.notify(MuxNotification::PaneRemoved(
        terminal_lib::mux::PaneId::from(1),
    ));

    // 处理跨线程通知（如果有的话）
    mux.process_notifications();

    // 给通知处理一些时间
    std::thread::sleep(std::time::Duration::from_millis(10));

    // 验证通知被接收
    let notifications = received_notifications.lock().unwrap();
    assert_eq!(notifications.len(), 3);

    // 验证通知类型
    match &notifications[0] {
        MuxNotification::PaneAdded(pane_id) => {
            assert_eq!(pane_id.as_u32(), 1);
        }
        _ => panic!("期望 PaneAdded 通知"),
    }

    match &notifications[1] {
        MuxNotification::PaneOutput { pane_id, data } => {
            assert_eq!(pane_id.as_u32(), 1);
            assert_eq!(data.as_ref(), b"test output");
        }
        _ => panic!("期望 PaneOutput 通知"),
    }

    match &notifications[2] {
        MuxNotification::PaneRemoved(pane_id) => {
            assert_eq!(pane_id.as_u32(), 1);
        }
        _ => panic!("期望 PaneRemoved 通知"),
    }
}
