//! 核心基础架构测试

#[cfg(test)]
mod tests {
    use terminal_lib::mux::{MuxError, MuxResult, PaneError, PaneId, PtySize};

    #[test]
    fn test_pane_id_creation() {
        let pane_id = PaneId::new(42);
        assert_eq!(pane_id.as_u32(), 42);

        let pane_id_from_u32: PaneId = 123.into();
        assert_eq!(pane_id_from_u32.as_u32(), 123);

        let u32_from_pane_id: u32 = pane_id.into();
        assert_eq!(u32_from_pane_id, 42);
    }

    #[test]
    fn test_pty_size_creation() {
        let size = PtySize::new(24, 80);
        assert_eq!(size.rows, 24);
        assert_eq!(size.cols, 80);
        assert_eq!(size.pixel_width, 0);
        assert_eq!(size.pixel_height, 0);

        let size_with_pixels = PtySize::with_pixels(30, 100, 800, 600);
        assert_eq!(size_with_pixels.rows, 30);
        assert_eq!(size_with_pixels.cols, 100);
        assert_eq!(size_with_pixels.pixel_width, 800);
        assert_eq!(size_with_pixels.pixel_height, 600);

        let default_size = PtySize::default();
        assert_eq!(default_size.rows, 24);
        assert_eq!(default_size.cols, 80);
    }

    #[test]
    fn test_error_conversion() {
        let pane_error = PaneError::PaneClosed;
        let mux_error: MuxError = pane_error.into();

        match mux_error {
            MuxError::Internal(msg) => assert!(msg.contains("面板已关闭")),
            _ => panic!("Expected Internal error"),
        }
    }

    #[test]
    fn test_error_recoverability() {
        let recoverable_error = MuxError::PtyError("test".to_string());
        assert!(recoverable_error.is_recoverable());

        let non_recoverable_error = MuxError::PaneNotFound(PaneId::new(1));
        assert!(!non_recoverable_error.is_recoverable());
    }

    #[test]
    fn test_error_severity() {
        let warning_error = MuxError::PaneNotFound(PaneId::new(1));
        assert_eq!(warning_error.severity().as_str(), "WARN");

        let critical_error = MuxError::Internal("test".to_string());
        assert_eq!(critical_error.severity().as_str(), "CRITICAL");
    }

    #[test]
    fn test_result_types() {
        let success: MuxResult<u32> = Ok(42);
        assert!(success.is_ok());

        let error: MuxResult<u32> = Err(MuxError::Internal("test".to_string()));
        assert!(error.is_err());
    }

    #[test]
    fn test_string_to_error_conversion() {
        let error: MuxError = "test error".into();
        match error {
            MuxError::Internal(msg) => assert_eq!(msg, "test error"),
            _ => panic!("Expected Internal error"),
        }

        let error: MuxError = String::from("another test").into();
        match error {
            MuxError::Internal(msg) => assert_eq!(msg, "another test"),
            _ => panic!("Expected Internal error"),
        }
    }
}
