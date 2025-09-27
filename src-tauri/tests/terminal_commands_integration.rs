/*!
 * 终端上下文命令集成测试
 *
 * 测试终端上下文管理的核心功能，包括：
 * - 活跃终端注册表的集成测试
 * - 终端上下文服务的集成测试
 * - 错误处理和边界条件测试
 */

use std::sync::Arc;
use terminal_lib::mux::{PaneId, TerminalMux};
use terminal_lib::shell::ShellIntegrationManager;
use terminal_lib::terminal::{
    commands::TerminalContextState, ActiveTerminalContextRegistry, TerminalContextService,
};

/// 创建测试用的终端上下文状态
fn create_test_context_state() -> TerminalContextState {
    let registry = Arc::new(ActiveTerminalContextRegistry::new());
    let shell_integration = Arc::new(ShellIntegrationManager::new().unwrap());
    let terminal_mux = Arc::new(TerminalMux::new());
    let context_service = Arc::new(TerminalContextService::new(
        registry.clone(),
        shell_integration,
        terminal_mux,
    ));

    TerminalContextState::new(registry, context_service)
}

#[tokio::test]
async fn test_terminal_context_state_creation() {
    let state = create_test_context_state();

    // 验证状态创建成功
    assert_eq!(state.registry().terminal_context_get_active_pane(), None);

    // 验证缓存统计初始状态
    let cache_stats = state.context_service().get_cache_stats();
    assert_eq!(cache_stats.total_entries, 0);
    assert_eq!(cache_stats.hit_count, 0);
    assert_eq!(cache_stats.miss_count, 0);
}

#[tokio::test]
async fn test_active_pane_management_integration() {
    let state = create_test_context_state();
    let pane_id = PaneId::new(123);

    // 初始状态验证
    assert_eq!(state.registry().terminal_context_get_active_pane(), None);
    assert!(!state.registry().terminal_context_is_pane_active(pane_id));

    // 设置活跃终端
    let result = state.registry().terminal_context_set_active_pane(pane_id);
    assert!(result.is_ok(), "设置活跃终端应该成功");

    // 验证活跃终端状态
    assert_eq!(
        state.registry().terminal_context_get_active_pane(),
        Some(pane_id)
    );
    assert!(state.registry().terminal_context_is_pane_active(pane_id));

    // 验证其他面板不是活跃的
    let other_pane = PaneId::new(456);
    assert!(!state.registry().terminal_context_is_pane_active(other_pane));

    // 清除活跃终端
    let result = state.registry().terminal_context_clear_active_pane();
    assert!(result.is_ok(), "清除活跃终端应该成功");

    // 验证清除后的状态
    assert_eq!(state.registry().terminal_context_get_active_pane(), None);
    assert!(!state.registry().terminal_context_is_pane_active(pane_id));
}

#[tokio::test]
async fn test_terminal_context_service_integration() {
    let state = create_test_context_state();
    let pane_id = PaneId::new(789);

    // 测试没有活跃终端时的上下文查询
    let result = state.context_service().get_active_context().await;
    assert!(result.is_err());

    // 测试回退逻辑
    let result = state
        .context_service()
        .get_context_with_fallback(None)
        .await;
    assert!(result.is_ok(), "回退逻辑应该返回默认上下文");

    let context = result.unwrap();
    assert_eq!(context.current_working_directory, Some("~".to_string()));
    assert!(matches!(
        context.shell_type,
        Some(terminal_lib::terminal::ShellType::Bash)
    ));
    assert!(!context.shell_integration_enabled);

    // 设置活跃终端后测试
    state
        .registry()
        .terminal_context_set_active_pane(pane_id)
        .unwrap();

    // 测试获取活跃终端上下文（面板不存在于mux中，应该失败）
    let result = state.context_service().get_active_context().await;
    assert!(result.is_err());

    // 测试使用回退逻辑获取上下文
    let result = state
        .context_service()
        .get_context_with_fallback(Some(pane_id))
        .await;
    assert!(result.is_ok(), "回退逻辑应该成功");

    let context = result.unwrap();
    assert_eq!(context.current_working_directory, Some("~".to_string()));
}

#[tokio::test]
async fn test_context_cache_operations() {
    let state = create_test_context_state();
    let pane_id = PaneId::new(321);

    // 初始缓存状态
    let stats = state.context_service().get_cache_stats();
    assert_eq!(stats.total_entries, 0);

    // 测试缓存失效操作
    state.context_service().invalidate_cache(pane_id);

    // 测试清除所有缓存
    state.context_service().clear_all_cache();

    // 验证缓存统计
    let stats = state.context_service().get_cache_stats();
    assert_eq!(stats.total_entries, 0);
}

#[tokio::test]
async fn test_registry_statistics() {
    let state = create_test_context_state();
    let pane_id = PaneId::new(654);

    // 初始统计
    let stats = state.registry().get_stats();
    assert_eq!(stats.global_active_pane, None);
    assert_eq!(stats.window_active_pane_count, 0);

    // 设置活跃终端后的统计
    state
        .registry()
        .terminal_context_set_active_pane(pane_id)
        .unwrap();

    let stats = state.registry().get_stats();
    assert_eq!(stats.global_active_pane, Some(pane_id));
    assert_eq!(stats.window_active_pane_count, 0); // 窗口级别的活跃终端暂未使用
}

#[tokio::test]
async fn test_concurrent_active_pane_operations() {
    let state = Arc::new(create_test_context_state());
    let pane_ids = vec![PaneId::new(100), PaneId::new(200), PaneId::new(300)];

    // 并发设置不同的活跃终端
    let mut handles = Vec::new();
    for pane_id in pane_ids.clone() {
        let state_clone = Arc::clone(&state);
        let handle = tokio::spawn(async move {
            // 每个任务都尝试设置自己的面板为活跃
            let set_result = state_clone
                .registry()
                .terminal_context_set_active_pane(pane_id);

            // 检查是否成功设置
            let is_active = state_clone
                .registry()
                .terminal_context_is_pane_active(pane_id);

            // 尝试获取上下文
            let context_result = state_clone
                .context_service()
                .get_context_with_fallback(Some(pane_id))
                .await;

            (
                pane_id,
                set_result.is_ok(),
                is_active,
                context_result.is_ok(),
            )
        });
        handles.push(handle);
    }

    // 等待所有任务完成
    let results = futures::future::join_all(handles).await;

    // 验证至少有一个操作成功
    let successful_sets = results.iter().filter(|r| r.as_ref().unwrap().1).count();
    assert!(successful_sets > 0, "至少应该有一个设置操作成功");

    // 验证最终状态一致性
    let final_active_pane = state.registry().terminal_context_get_active_pane();
    if let Some(active_id) = final_active_pane {
        assert!(
            pane_ids.contains(&active_id),
            "最终活跃终端应该是设置的面板之一"
        );
    }
}

#[tokio::test]
async fn test_error_handling_and_recovery() {
    let state = create_test_context_state();
    let invalid_pane = PaneId::new(999);

    // 测试获取不存在面板的上下文
    let result = state
        .context_service()
        .get_context_by_pane(invalid_pane)
        .await;
    assert!(result.is_err());

    // 测试回退逻辑能够处理错误
    let result = state
        .context_service()
        .get_context_with_fallback(Some(invalid_pane))
        .await;
    assert!(result.is_ok(), "回退逻辑应该能处理不存在的面板");

    let context = result.unwrap();
    assert_eq!(context.current_working_directory, Some("~".to_string()));
    assert!(matches!(
        context.shell_type,
        Some(terminal_lib::terminal::ShellType::Bash)
    ));
}

#[tokio::test]
async fn test_complete_workflow_integration() {
    let state = create_test_context_state();
    let pane_id = PaneId::new(555);

    // 完整的工作流程测试

    // 1. 初始状态验证
    assert_eq!(state.registry().terminal_context_get_active_pane(), None);
    assert!(!state.registry().terminal_context_is_pane_active(pane_id));

    // 2. 设置活跃终端
    state
        .registry()
        .terminal_context_set_active_pane(pane_id)
        .unwrap();
    assert_eq!(
        state.registry().terminal_context_get_active_pane(),
        Some(pane_id)
    );
    assert!(state.registry().terminal_context_is_pane_active(pane_id));

    // 3. 获取终端上下文（使用回退逻辑）
    let context = state
        .context_service()
        .get_context_with_fallback(Some(pane_id))
        .await
        .unwrap();
    assert_eq!(context.current_working_directory, Some("~".to_string()));

    // 4. 缓存操作
    state.context_service().invalidate_cache(pane_id);
    let stats = state.context_service().get_cache_stats();
    assert_eq!(stats.total_entries, 0);

    // 5. 清除活跃终端
    state
        .registry()
        .terminal_context_clear_active_pane()
        .unwrap();
    assert_eq!(state.registry().terminal_context_get_active_pane(), None);
    assert!(!state.registry().terminal_context_is_pane_active(pane_id));

    // 6. 验证清除后仍能获取默认上下文
    let context = state
        .context_service()
        .get_context_with_fallback(None)
        .await
        .unwrap();
    assert_eq!(context.current_working_directory, Some("~".to_string()));
}

#[tokio::test]
async fn test_event_system_integration() {
    let state = create_test_context_state();
    let pane_id = PaneId::new(777);

    // 订阅事件
    let mut event_receiver = state.registry().subscribe_events();

    // 设置活跃终端应该触发事件
    state
        .registry()
        .terminal_context_set_active_pane(pane_id)
        .unwrap();

    // 尝试接收事件（使用超时避免测试挂起）
    let event_result =
        tokio::time::timeout(std::time::Duration::from_millis(100), event_receiver.recv()).await;

    assert!(event_result.is_ok(), "应该在超时前收到事件");

    if let Ok(Ok(event)) = event_result {
        match event {
            terminal_lib::terminal::TerminalContextEvent::ActivePaneChanged {
                old_pane_id,
                new_pane_id,
            } => {
                assert_eq!(old_pane_id, None);
                assert_eq!(new_pane_id, Some(pane_id));
            }
            _ => panic!("收到了错误的事件类型"),
        }
    }
}

#[tokio::test]
async fn test_state_access_methods() {
    let registry = Arc::new(ActiveTerminalContextRegistry::new());
    let shell_integration = Arc::new(ShellIntegrationManager::new().unwrap());
    let terminal_mux = Arc::new(TerminalMux::new());
    let context_service = Arc::new(TerminalContextService::new(
        registry.clone(),
        shell_integration,
        terminal_mux,
    ));

    let state = TerminalContextState::new(registry.clone(), context_service.clone());

    // 验证状态访问方法
    assert!(Arc::ptr_eq(state.registry(), &registry));
    assert!(Arc::ptr_eq(state.context_service(), &context_service));
}
