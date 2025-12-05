//! 高性能环形输出缓冲区

/// 环形输出缓冲区
///
/// 固定大小，避免内存无限增长
pub struct OutputRingBuffer {
    /// 缓冲区数据
    buffer: Vec<u8>,
    /// 最大容量
    capacity: usize,
    /// 是否已溢出（数据被覆盖）
    overflowed: bool,
    /// 总写入字节数
    total_written: u64,
}

impl OutputRingBuffer {
    /// 创建新的环形缓冲区
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            capacity,
            overflowed: false,
            total_written: 0,
        }
    }

    /// 写入数据
    pub fn write(&mut self, data: &[u8]) {
        self.total_written += data.len() as u64;

        if data.len() >= self.capacity {
            // 数据比缓冲区大，只保留最后 capacity 字节
            self.buffer.clear();
            self.buffer
                .extend_from_slice(&data[data.len() - self.capacity..]);
            self.overflowed = true;
        } else if self.buffer.len() + data.len() > self.capacity {
            // 需要丢弃旧数据
            let overflow = self.buffer.len() + data.len() - self.capacity;
            self.buffer.drain(0..overflow);
            self.buffer.extend_from_slice(data);
            self.overflowed = true;
        } else {
            // 直接追加
            self.buffer.extend_from_slice(data);
        }
    }

    /// 写入字符串
    pub fn write_str(&mut self, s: &str) {
        self.write(s.as_bytes());
    }

    /// 获取当前内容
    pub fn content(&self) -> &[u8] {
        &self.buffer
    }

    /// 获取当前内容为字符串
    pub fn content_string(&self) -> String {
        String::from_utf8_lossy(&self.buffer).to_string()
    }

    /// 是否已溢出
    pub fn is_overflowed(&self) -> bool {
        self.overflowed
    }

    /// 总写入字节数
    pub fn total_written(&self) -> u64 {
        self.total_written
    }

    /// 当前缓冲区大小
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// 缓冲区是否为空
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// 清空缓冲区
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.overflowed = false;
        self.total_written = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_write() {
        let mut buf = OutputRingBuffer::new(100);
        buf.write_str("hello");
        assert_eq!(buf.content_string(), "hello");
        assert!(!buf.is_overflowed());
    }

    #[test]
    fn test_overflow() {
        let mut buf = OutputRingBuffer::new(10);
        buf.write_str("hello world!"); // 12 bytes > 10
        assert_eq!(buf.len(), 10);
        assert!(buf.is_overflowed());
        assert_eq!(buf.content_string(), "lo world!");
    }

    #[test]
    fn test_incremental_overflow() {
        let mut buf = OutputRingBuffer::new(10);
        buf.write_str("hello"); // 5 bytes
        buf.write_str("world!"); // 6 bytes, total 11 > 10
        assert!(buf.is_overflowed());
        assert_eq!(buf.len(), 10);
    }
}
