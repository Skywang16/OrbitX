use std::mem::MaybeUninit;
use std::ops::Index;

/// 固定大小环形缓冲区
///
/// 性能：O(1) push，固定内存 N * sizeof(T)，永不重新分配
pub struct MessageRingBuffer<T, const N: usize> {
    data: Box<[MaybeUninit<T>; N]>,
    head: usize,
    len: usize,
}

impl<T, const N: usize> MessageRingBuffer<T, N> {
    pub fn new() -> Self {
        Self {
            data: Box::new(unsafe { MaybeUninit::uninit().assume_init() }),
            head: 0,
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn capacity(&self) -> usize {
        N
    }

    pub fn push(&mut self, item: T) {
        if self.len < N {
            unsafe {
                self.data[self.head].as_mut_ptr().write(item);
            }
            self.len += 1;
        } else {
            unsafe {
                std::ptr::drop_in_place(self.data[self.head].as_mut_ptr());
                self.data[self.head].as_mut_ptr().write(item);
            }
        }
        self.head = (self.head + 1) % N;
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len {
            return None;
        }
        let start = if self.len < N { 0 } else { self.head };
        let actual_index = (start + index) % N;
        Some(unsafe { &*self.data[actual_index].as_ptr() })
    }

    /// 迭代器（按插入顺序）
    pub fn iter(&self) -> RingBufferIter<'_, T, N> {
        RingBufferIter {
            buffer: self,
            index: 0,
        }
    }

    /// 转换为Vec（深拷贝）
    pub fn to_vec(&self) -> Vec<T>
    where
        T: Clone,
    {
        self.iter().cloned().collect()
    }

    pub fn clear(&mut self) {
        for i in 0..self.len {
            let start = if self.len < N { 0 } else { self.head };
            let actual_index = (start + i) % N;
            unsafe {
                std::ptr::drop_in_place(self.data[actual_index].as_mut_ptr());
            }
        }
        self.head = 0;
        self.len = 0;
    }
}

impl<T, const N: usize> Default for MessageRingBuffer<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> Drop for MessageRingBuffer<T, N> {
    fn drop(&mut self) {
        self.clear();
    }
}

impl<T, const N: usize> Index<usize> for MessageRingBuffer<T, N> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).expect("index out of bounds")
    }
}

pub struct RingBufferIter<'a, T, const N: usize> {
    buffer: &'a MessageRingBuffer<T, N>,
    index: usize,
}

impl<'a, T, const N: usize> Iterator for RingBufferIter<'a, T, N> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.buffer.len {
            let item = self.buffer.get(self.index);
            self.index += 1;
            item
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.buffer.len - self.index;
        (remaining, Some(remaining))
    }
}

impl<'a, T, const N: usize> ExactSizeIterator for RingBufferIter<'a, T, N> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() {
        let mut buffer = MessageRingBuffer::<i32, 5>::new();
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());

        buffer.push(1);
        buffer.push(2);
        buffer.push(3);
        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.get(0), Some(&1));
        assert_eq!(buffer.get(1), Some(&2));
        assert_eq!(buffer.get(2), Some(&3));
    }

    #[test]
    fn test_wrap_around() {
        let mut buffer = MessageRingBuffer::<i32, 3>::new();
        buffer.push(1);
        buffer.push(2);
        buffer.push(3);
        buffer.push(4); // 覆盖 1
        buffer.push(5); // 覆盖 2

        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.get(0), Some(&3));
        assert_eq!(buffer.get(1), Some(&4));
        assert_eq!(buffer.get(2), Some(&5));
    }

    #[test]
    fn test_iter() {
        let mut buffer = MessageRingBuffer::<i32, 5>::new();
        buffer.push(1);
        buffer.push(2);
        buffer.push(3);

        let vec: Vec<_> = buffer.iter().copied().collect();
        assert_eq!(vec, vec![1, 2, 3]);
    }

    #[test]
    fn test_clear() {
        let mut buffer = MessageRingBuffer::<String, 5>::new();
        buffer.push("hello".to_string());
        buffer.push("world".to_string());
        assert_eq!(buffer.len(), 2);

        buffer.clear();
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
    }
}
