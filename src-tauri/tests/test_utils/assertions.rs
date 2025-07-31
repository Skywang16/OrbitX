/*!
 * 统一断言工具
 * 
 * 提供增强的断言宏和函数，用于更清晰的测试验证
 */

/// 增强的相等断言，提供更详细的错误信息
#[macro_export]
macro_rules! assert_eq_detailed {
    ($left:expr, $right:expr, $context:expr) => {
        match (&$left, &$right) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    panic!(
                        "断言失败: {} \n左值: {:?}\n右值: {:?}",
                        $context, left_val, right_val
                    );
                }
            }
        }
    };
}

/// 增强的不等断言
#[macro_export]
macro_rules! assert_ne_detailed {
    ($left:expr, $right:expr, $context:expr) => {
        match (&$left, &$right) {
            (left_val, right_val) => {
                if *left_val == *right_val {
                    panic!(
                        "断言失败: {} \n值不应该相等: {:?}",
                        $context, left_val
                    );
                }
            }
        }
    };
}

/// 断言操作成功
#[macro_export]
macro_rules! assert_ok {
    ($result:expr) => {
        match $result {
            Ok(val) => val,
            Err(e) => panic!("期望操作成功，但失败了: {:?}", e),
        }
    };
    ($result:expr, $context:expr) => {
        match $result {
            Ok(val) => val,
            Err(e) => panic!("期望操作成功，但失败了 ({}): {:?}", $context, e),
        }
    };
}

/// 断言操作失败
#[macro_export]
macro_rules! assert_err {
    ($result:expr) => {
        match $result {
            Ok(val) => panic!("期望操作失败，但成功了: {:?}", val),
            Err(e) => e,
        }
    };
    ($result:expr, $context:expr) => {
        match $result {
            Ok(val) => panic!("期望操作失败，但成功了 ({}): {:?}", $context, val),
            Err(e) => e,
        }
    };
}

/// 断言错误包含特定消息
#[macro_export]
macro_rules! assert_error_contains {
    ($result:expr, $expected_msg:expr) => {
        match $result {
            Ok(val) => panic!("期望操作失败，但成功了: {:?}", val),
            Err(e) => {
                let error_msg = format!("{:?}", e);
                assert!(
                    error_msg.contains($expected_msg),
                    "错误消息应该包含 '{}', 实际错误: {}",
                    $expected_msg, error_msg
                );
            }
        }
    };
}

/// 断言错误类型匹配
#[macro_export]
macro_rules! assert_error_type {
    ($result:expr, $error_type:pat) => {
        match $result {
            Ok(val) => panic!("期望操作失败，但成功了: {:?}", val),
            Err(e) => {
                assert!(
                    matches!(e, $error_type),
                    "错误类型不匹配，期望: {}, 实际: {:?}",
                    stringify!($error_type), e
                );
            }
        }
    };
}

/// 断言Option为Some
#[macro_export]
macro_rules! assert_some {
    ($option:expr) => {
        match $option {
            Some(val) => val,
            None => panic!("期望Some值，但得到None"),
        }
    };
    ($option:expr, $context:expr) => {
        match $option {
            Some(val) => val,
            None => panic!("期望Some值，但得到None ({})", $context),
        }
    };
}

/// 断言Option为None
#[macro_export]
macro_rules! assert_none {
    ($option:expr) => {
        match $option {
            None => (),
            Some(val) => panic!("期望None，但得到Some: {:?}", val),
        }
    };
    ($option:expr, $context:expr) => {
        match $option {
            None => (),
            Some(val) => panic!("期望None，但得到Some ({}): {:?}", $context, val),
        }
    };
}

/// 断言集合包含元素
#[macro_export]
macro_rules! assert_contains {
    ($collection:expr, $item:expr) => {
        assert!(
            $collection.contains(&$item),
            "集合应该包含元素: {:?}",
            $item
        );
    };
    ($collection:expr, $item:expr, $context:expr) => {
        assert!(
            $collection.contains(&$item),
            "集合应该包含元素 ({}): {:?}",
            $context, $item
        );
    };
}

/// 断言集合不包含元素
#[macro_export]
macro_rules! assert_not_contains {
    ($collection:expr, $item:expr) => {
        assert!(
            !$collection.contains(&$item),
            "集合不应该包含元素: {:?}",
            $item
        );
    };
    ($collection:expr, $item:expr, $context:expr) => {
        assert!(
            !$collection.contains(&$item),
            "集合不应该包含元素 ({}): {:?}",
            $context, $item
        );
    };
}

/// 断言集合长度
#[macro_export]
macro_rules! assert_len {
    ($collection:expr, $expected_len:expr) => {
        assert_eq!(
            $collection.len(),
            $expected_len,
            "集合长度不匹配，期望: {}, 实际: {}",
            $expected_len,
            $collection.len()
        );
    };
    ($collection:expr, $expected_len:expr, $context:expr) => {
        assert_eq!(
            $collection.len(),
            $expected_len,
            "集合长度不匹配 ({}), 期望: {}, 实际: {}",
            $context,
            $expected_len,
            $collection.len()
        );
    };
}

/// 断言集合为空
#[macro_export]
macro_rules! assert_empty {
    ($collection:expr) => {
        assert!(
            $collection.is_empty(),
            "集合应该为空，但包含 {} 个元素",
            $collection.len()
        );
    };
    ($collection:expr, $context:expr) => {
        assert!(
            $collection.is_empty(),
            "集合应该为空 ({}), 但包含 {} 个元素",
            $context,
            $collection.len()
        );
    };
}

/// 断言集合不为空
#[macro_export]
macro_rules! assert_not_empty {
    ($collection:expr) => {
        assert!(
            !$collection.is_empty(),
            "集合不应该为空"
        );
    };
    ($collection:expr, $context:expr) => {
        assert!(
            !$collection.is_empty(),
            "集合不应该为空 ({})",
            $context
        );
    };
}

/// 断言字符串包含子串
#[macro_export]
macro_rules! assert_str_contains {
    ($string:expr, $substring:expr) => {
        assert!(
            $string.contains($substring),
            "字符串应该包含子串 '{}', 实际字符串: '{}'",
            $substring, $string
        );
    };
    ($string:expr, $substring:expr, $context:expr) => {
        assert!(
            $string.contains($substring),
            "字符串应该包含子串 '{}' ({}), 实际字符串: '{}'",
            $substring, $context, $string
        );
    };
}

/// 断言字符串不包含子串
#[macro_export]
macro_rules! assert_str_not_contains {
    ($string:expr, $substring:expr) => {
        assert!(
            !$string.contains($substring),
            "字符串不应该包含子串 '{}', 实际字符串: '{}'",
            $substring, $string
        );
    };
    ($string:expr, $substring:expr, $context:expr) => {
        assert!(
            !$string.contains($substring),
            "字符串不应该包含子串 '{}' ({}), 实际字符串: '{}'",
            $substring, $context, $string
        );
    };
}

/// 断言数值在范围内
#[macro_export]
macro_rules! assert_in_range {
    ($value:expr, $min:expr, $max:expr) => {
        assert!(
            $value >= $min && $value <= $max,
            "值应该在范围 [{}, {}] 内, 实际值: {}",
            $min, $max, $value
        );
    };
    ($value:expr, $min:expr, $max:expr, $context:expr) => {
        assert!(
            $value >= $min && $value <= $max,
            "值应该在范围 [{}, {}] 内 ({}), 实际值: {}",
            $min, $max, $context, $value
        );
    };
}

/// 断言时间差在容忍范围内
#[macro_export]
macro_rules! assert_duration_near {
    ($actual:expr, $expected:expr, $tolerance:expr) => {
        let diff = if $actual > $expected {
            $actual - $expected
        } else {
            $expected - $actual
        };
        assert!(
            diff <= $tolerance,
            "时间差超出容忍范围，期望: {:?}, 实际: {:?}, 容忍: {:?}, 差值: {:?}",
            $expected, $actual, $tolerance, diff
        );
    };
}

/// 断言浮点数近似相等
#[macro_export]
macro_rules! assert_float_eq {
    ($left:expr, $right:expr, $epsilon:expr) => {
        let diff = ($left - $right).abs();
        assert!(
            diff < $epsilon,
            "浮点数不相等，左值: {}, 右值: {}, 差值: {}, 容忍: {}",
            $left, $right, diff, $epsilon
        );
    };
}

/// 断言最终条件（带重试）
#[macro_export]
macro_rules! assert_eventually {
    ($condition:expr, $timeout:expr) => {
        assert_eventually!($condition, $timeout, std::time::Duration::from_millis(100))
    };
    ($condition:expr, $timeout:expr, $interval:expr) => {{
        let start = std::time::Instant::now();
        loop {
            if $condition {
                break;
            }
            if start.elapsed() > $timeout {
                panic!("条件在超时时间内未满足: {}", stringify!($condition));
            }
            std::thread::sleep($interval);
        }
    }};
}

/// 异步断言最终条件
#[macro_export]
macro_rules! assert_eventually_async {
    ($condition:expr, $timeout:expr) => {
        assert_eventually_async!($condition, $timeout, std::time::Duration::from_millis(100))
    };
    ($condition:expr, $timeout:expr, $interval:expr) => {{
        let start = std::time::Instant::now();
        loop {
            if $condition.await {
                break;
            }
            if start.elapsed() > $timeout {
                panic!("异步条件在超时时间内未满足: {}", stringify!($condition));
            }
            tokio::time::sleep($interval).await;
        }
    }};
}
