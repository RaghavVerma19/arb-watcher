#[derive(Clone, Debug)]
pub struct RingBuffer<T> {
    buf: Vec<Option<T>>,
    head: usize,
    len: usize,
}

impl<T: Clone> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            buf: vec![None; capacity],
            head: 0,
            len: 0,
        }
    }

    pub fn push(&mut self, value: T) {
        self.buf[self.head] = Some(value);
        self.head = (self.head + 1) % self.buf.len();
        if self.len < self.buf.len() {
            self.len += 1;
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn capacity(&self) -> usize {
        self.buf.len()
    }

    /// Iterate oldest -> newest.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        let cap = self.buf.len();
        let head = self.head;
        let len = self.len;
        (0..len).map(move |i| {
            let pos = (head + cap - len + i) % cap;
            self.buf[pos].as_ref().unwrap()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_iter_order() {
        let mut rb: RingBuffer<i32> = RingBuffer::new(3);
        rb.push(1);
        rb.push(2);
        rb.push(3);
        assert_eq!(rb.iter().cloned().collect::<Vec<_>>(), vec![1, 2, 3]);
    }

    #[test]
    fn overwrites_oldest() {
        let mut rb: RingBuffer<i32> = RingBuffer::new(3);
        rb.push(1);
        rb.push(2);
        rb.push(3);
        rb.push(4);
        assert_eq!(rb.len(), 3);
        assert_eq!(rb.iter().cloned().collect::<Vec<_>>(), vec![2, 3, 4]);
    }

    #[test]
    fn empty_has_no_elements() {
        let rb: RingBuffer<i32> = RingBuffer::new(5);
        assert!(rb.is_empty());
        assert_eq!(rb.iter().count(), 0);
    }
}
