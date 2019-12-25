use std::sync::atomic::{AtomicU32, Ordering};

#[repr(align(64))]
#[repr(C)]
#[derive(Debug)]
pub(crate) struct Cursor {
    pub head: AtomicU32,
    pub tail: AtomicU32,
    size: u32,
    mask: u32,
}

impl Cursor {
    #[inline(always)]
    pub fn new(size: u32) -> Self {
        Cursor{ 
            head: AtomicU32::new(0), 
            tail: AtomicU32::new(0), 
            size: size,
            mask: size-1 
        }
    }

    #[inline]
    pub fn next(&mut self, head: u32) -> u32 {
        head.wrapping_add(1) & self.mask
    }

    #[inline]
    pub fn front(&mut self) -> u32 {
        self.head.load(Ordering::SeqCst)
    }
    #[inline]
    pub fn back(&mut self) -> u32 {
        self.tail.load(Ordering::SeqCst)
    }

    #[inline]
    pub fn reached(&mut self, head: u32) -> bool {
        assert!(head < self.size);
        
        self.tail.load(Ordering::SeqCst) == head
    }

    #[inline]
    pub fn exchange_front(&mut self, head: u32, next: u32) -> bool {
        assert!(head < self.size && next < self.size);

        self.head
        .compare_exchange(head, next, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    }

    #[inline]
    pub fn exchange_back(&mut self, tail: u32, next: u32) -> bool {
        assert!(tail < self.size && next < self.size);

        self.tail
        .compare_exchange(tail, next, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constructor() {
        const SZ : u32 = 8;

        let cursor = Cursor::new(SZ);
        assert_eq!(cursor.head.load(Ordering::Acquire), 0);
        assert_eq!(cursor.tail.load(Ordering::Acquire), 0);
        assert_eq!(cursor.size, SZ);
        assert_eq!(cursor.mask, SZ-1);
    }

    #[test]
    fn test_next() {
        const SZ : u32 = 8;
        let mut cursor = Cursor::new(SZ);
        let h = cursor.front();
        let n = cursor.next(h);

        assert_eq!(h, 0);
        assert_eq!(n, 1);
    }

    #[test]
    fn test_exchange() {
        const SZ : u32 = 8;
        let mut cursor = Cursor::new(SZ);
        let h = cursor.front();
        let n = cursor.next(h);

        assert_eq!(h, 0);
        assert_eq!(n, 1);

        cursor.exchange_front(h, n);
        assert_eq!(cursor.head.load(Ordering::Acquire), n);
        let h = cursor.front();
        let n = cursor.next(h);

        assert_eq!(h, 1);
        assert_eq!(n, 2);
    }

    #[test]
    fn test_overflow() {
        const SZ : u32 = 8;
        let mut cursor = Cursor::new(SZ);
        cursor.exchange_front(0, 7);
        
        let h = cursor.front();
        let n = cursor.next(h);
        
        assert_eq!(h, 7);
        assert_eq!(n, 0);
    }
}
