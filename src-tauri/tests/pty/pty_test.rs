use terminal_lib::mux::{LocalPane, Pane, PaneId, PtySize, TerminalConfig};

#[test]
fn test_pty_creation() {
    let pane_id = PaneId::new(1);
    let size = PtySize::new(24, 80);

    // 测试PTY创建
    let result = LocalPane::new(pane_id, size);
    assert!(result.is_ok(), "PTY创建应该成功");

    let pane = result.unwrap();
    assert_eq!(pane.pane_id(), pane_id);
    assert_eq!(pane.get_size().rows, 24);
    assert_eq!(pane.get_size().cols, 80);
    assert!(!pane.is_dead());
}

#[test]
fn test_pty_with_config() {
    let pane_id = PaneId::new(2);
    let size = PtySize::new(30, 100);
    let config = TerminalConfig::default();

    // 测试使用配置创建PTY
    let result = LocalPane::new_with_config(pane_id, size, &config);
    assert!(result.is_ok(), "使用配置创建PTY应该成功");

    let pane = result.unwrap();
    assert_eq!(pane.pane_id(), pane_id);
    assert_eq!(pane.get_size().rows, 30);
    assert_eq!(pane.get_size().cols, 100);
}

#[test]
fn test_pty_write() {
    let pane_id = PaneId::new(3);
    let size = PtySize::new(24, 80);

    let pane = LocalPane::new(pane_id, size).expect("PTY创建失败");

    // 测试写入功能
    let result = pane.write(b"echo hello\n");
    assert!(result.is_ok(), "PTY写入应该成功");

    // 测试字符串写入
    let result = pane.write_str("ls -la\n");
    assert!(result.is_ok(), "字符串写入应该成功");

    // 测试行写入
    let result = pane.write_line("pwd");
    assert!(result.is_ok(), "行写入应该成功");
}

#[test]
fn test_pty_resize() {
    let pane_id = PaneId::new(4);
    let size = PtySize::new(24, 80);

    let pane = LocalPane::new(pane_id, size).expect("PTY创建失败");

    // 测试调整大小
    let new_size = PtySize::new(30, 120);
    let result = pane.resize(new_size);
    assert!(result.is_ok(), "PTY调整大小应该成功");

    // 验证大小已更新
    let current_size = pane.get_size();
    assert_eq!(current_size.rows, 30);
    assert_eq!(current_size.cols, 120);
}

#[test]
fn test_pty_control_sequences() {
    let pane_id = PaneId::new(5);
    let size = PtySize::new(24, 80);

    let pane = LocalPane::new(pane_id, size).expect("PTY创建失败");

    // 测试控制字符
    assert!(pane.send_control('c').is_ok(), "发送Ctrl+C应该成功");
    assert!(pane.send_control('d').is_ok(), "发送Ctrl+D应该成功");

    // 测试特殊键
    assert!(pane.send_key("Enter").is_ok(), "发送Enter键应该成功");
    assert!(pane.send_key("Tab").is_ok(), "发送Tab键应该成功");
    assert!(pane.send_key("Up").is_ok(), "发送上箭头键应该成功");
    assert!(pane.send_key("Down").is_ok(), "发送下箭头键应该成功");
}

#[test]
fn test_pty_reader() {
    let pane_id = PaneId::new(6);
    let size = PtySize::new(24, 80);

    let pane = LocalPane::new(pane_id, size).expect("PTY创建失败");

    // 测试获取读取器
    let result = pane.reader();
    assert!(result.is_ok(), "获取PTY读取器应该成功");
}

#[test]
fn test_pty_death_handling() {
    let pane_id = PaneId::new(7);
    let size = PtySize::new(24, 80);

    let pane = LocalPane::new(pane_id, size).expect("PTY创建失败");

    // 初始状态应该是活着的
    assert!(!pane.is_dead());

    // 标记为死亡
    pane.mark_dead();
    assert!(pane.is_dead());

    // 死亡后的操作应该失败
    assert!(pane.write(b"test").is_err());
    assert!(pane.resize(PtySize::new(25, 81)).is_err());
    assert!(pane.reader().is_err());
}
