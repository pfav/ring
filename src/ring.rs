use std::mem;
use std::sync::atomic::{AtomicUsize, Ordering, spin_loop_hint, compiler_fence};
use std::fmt::Debug;

use crate::cursor::Cursor;
use crate::buffer::Buffer;

#[repr(C)]
pub struct Ring<T> {
    prod: Cursor,
    cons: Cursor,
    inner: Buffer<T>,
    count: AtomicUsize,
}

impl<T: Debug> Ring<T> {
    #[inline]
    pub fn new(log2 : usize) -> Self {
        assert!(log2 > 1, "log2 must give greather than 1");
        assert!(mem::size_of::<T>() > 0, "value size must be greather than zero");

        let size : usize = 1 << log2;

        Ring{
            prod: Cursor::new(size as u32),
            cons: Cursor::new(size as u32),
            inner: Buffer::new(size),
            count: AtomicUsize::new(0),
        }
    }

    #[inline]
    pub fn single_enqueue(&mut self, value: T) -> Option<T> {
        let head = self.prod.front();
        let next = self.prod.next(head);

        if self.cons.reached(next) {
            return Some(value);
        }
            
        self.prod.head.store(next, Ordering::Release);

        self.inner.write(head as usize, value);
        
        self.prod.tail.store(next, Ordering::Release);
        
        None
    }

    #[inline]
    pub fn single_dequeue(&mut self) -> Option<T> {
        let head = self.cons.front();
        let next = self.cons.next(head);

        if self.prod.reached(head) {
            return None;
        }
            
        self.cons.head.store(next, Ordering::Release);

        let value = self.inner.read(head as usize);
        
        self.cons.tail.store(next, Ordering::Release);

        Some(value)
    }
    
    #[inline]
    pub fn multi_enqueue(&mut self, value: T) -> Option<T> {
        let mut head : u32;
        let mut next : u32;
        let mut tail : u32;
        
        loop {
            head = self.prod.front();
            tail = self.cons.back();
            
            compiler_fence(Ordering::SeqCst);

            next = self.prod.next(head);

            if next == tail {
                return Some(value);
            }
            
            if self.prod.exchange_front(head, next) { 
                break;
            }
            
            spin_loop_hint();
        };

        self.inner.write(head as usize, value);
        
        compiler_fence(Ordering::SeqCst);
        
        while !self.prod.exchange_back(head, next) {
            spin_loop_hint();
        }
        
        None
    }

    #[inline]
    pub fn multi_dequeue(&mut self) -> Option<T> {
        let mut head : u32;
        let mut next : u32;
        let mut tail : u32;

        loop {
            head = self.cons.front();
            tail = self.prod.back();
            
            compiler_fence(Ordering::SeqCst);

            next = self.cons.next(head);
            
            if head == tail {
                return None;
            }
            
            if self.cons.exchange_front(head, next){ 
                break
            }
            
            spin_loop_hint();
        };

        let value = self.inner.read(head as usize);
        
        compiler_fence(Ordering::SeqCst);
        
        while !self.cons.exchange_back(head, next) {
            spin_loop_hint();
        }

        Some(value)
    }
}

impl<T> Drop for Ring<T> {
    fn drop(&mut self) {
        for i in 0..self.inner.size {
            self.inner.drop_at(i)
        }
    }
}

unsafe impl<T> Sync for Ring<T> {}
unsafe impl<T> Send for Ring<T> {}

#[cfg(test)]
mod tests {
    use std::thread;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering, spin_loop_hint};
    use std::cell::UnsafeCell;
    use super::*;

    #[test]
    fn test_constructor() {
        let mut ring : Ring<u8> = Ring::new(8);
        assert_eq!(ring.single_dequeue(), None);
        assert_eq!(ring.multi_dequeue(), None);
    }

    #[test]
    fn test_single() {
        let mut ring : Ring<u8> = Ring::new(2);
        
        assert_eq!(ring.single_enqueue(0), None);
        assert_eq!(ring.single_enqueue(1), None);
        assert_eq!(ring.single_enqueue(2), None);
        assert_eq!(ring.single_enqueue(3), Some(3));

        assert_eq!(ring.single_dequeue(), Some(0));
        assert_eq!(ring.single_dequeue(), Some(1));
        assert_eq!(ring.single_dequeue(), Some(2));
        assert_eq!(ring.single_dequeue(), None);
    }

    #[test]
    fn test_multi() {
        let mut ring : Ring<u8> = Ring::new(2);
        
        assert_eq!(ring.multi_enqueue(0), None);
        assert_eq!(ring.multi_enqueue(1), None);
        assert_eq!(ring.multi_enqueue(2), None);
        assert_eq!(ring.multi_enqueue(3), Some(3));

        assert_eq!(ring.multi_dequeue(), Some(0));
        assert_eq!(ring.multi_dequeue(), Some(1));
        assert_eq!(ring.multi_dequeue(), Some(2));
        assert_eq!(ring.multi_dequeue(), None);
    }


    struct Wrapper<T> {
        inner: Arc<UnsafeCell<Ring<T>>>,
    }
    
    impl<T: Debug> Wrapper<T> {
        fn new(size: usize) -> Self {
            let inner = Arc::new(UnsafeCell::new(Ring::new(size)));
            Wrapper{ inner }
        }

        fn single_enqueue(&self, v: T) -> Option<T> { 
            self.ring().single_enqueue(v)
        }

        fn single_dequeue(&self) -> Option<T> { 
            self.ring().single_dequeue()
        }        

        fn multi_enqueue(&self, v: T) -> Option<T> { 
            self.ring().multi_enqueue(v)
        }
        fn multi_dequeue(&self) -> Option<T> { 
            self.ring().multi_dequeue()
        }                

        fn ring(&self) -> &mut Ring<T> {
            unsafe { &mut (*self.inner.get()) }
        }
        
        fn clone(&self) -> Self {
            Wrapper{ inner: Arc::clone(&self.inner) }
        }
    }

    unsafe impl<T> Sync for Wrapper<T> { }
    unsafe impl<T> Send for Wrapper<T> { }

    macro_rules! sum {
        ($n:expr, $ty:ty ) => {
            (0 .. $n).fold(0, |a, b| a + b)
        };
    }

    macro_rules! ring_test {
        ( $ty:ty,
          $PRODUCERS:expr,
          $CONSUMERS:expr,
          $N:expr, 
          $bitsize:expr,
          $prod:ident,
          $cons:ident
          ) => {{
        
            const PRODUCERS : usize = $PRODUCERS;
            const CONSUMERS : usize = $CONSUMERS;
            const N : usize = $N;
            const CN : usize = (N*PRODUCERS/CONSUMERS as usize);
            let result = Arc::new(AtomicUsize::new(0));

            let ring = Wrapper::new($bitsize);
            let mut prods = Vec::with_capacity(PRODUCERS);
            let mut cons = Vec::with_capacity(CONSUMERS);
            
            assert_eq!(N*PRODUCERS, CN*CONSUMERS);
            
            eprintln!("VALUES {} {}", N*PRODUCERS, CN*CONSUMERS);

            // producers
            for i in 0..PRODUCERS {
                let r = ring.clone();
                prods.push(thread::spawn(move || {
                    for j in 0..N {
                        let mut val = j;
                        'inner: loop {
                            match r.$prod(val as $ty) {
                                Some(v) => { 
                                    val = v as usize; 
                                    spin_loop_hint();
                                    continue
                                },
                                None => { 
                                    break 'inner;
                                }
                            }
                        }
                    }
                    eprintln!("producer {} DONE {}", i, N);
                }));
                
            }
            
            // consumers
            for i in 0..CONSUMERS {
                let r = ring.clone();
                let result = Arc::clone(&result);
                cons.push(thread::spawn(move || {
                    for _ in 0..CN {
                        'inner: loop {
                            match r.$cons() {
                                Some(v) => { 
                                    result.fetch_add(v as usize, Ordering::SeqCst);
                                    break 'inner 
                                },
                                None => {
                                    spin_loop_hint();
                                    continue;
                                }
                            }
                        }
                    }
                    eprintln!("consumer {} DONE {}", i, CN);
                }));
            }
        
            for prod in prods {
                prod.join().unwrap();
            }

            for con in cons {
                con.join().unwrap();
            }
            
            assert_eq!(result.load(Ordering::Acquire), PRODUCERS*sum!(N, $ty));
        }};
    }

    #[test]
    fn test_single_producer_single_consumer() {
        ring_test!(usize, 1, 1, 1_000_000, 2, single_enqueue, single_dequeue);
    }

    #[test]
    fn test_single_producer_multi_consumer() {
        ring_test!(usize, 1, 2, 1_000, 8, multi_enqueue, multi_dequeue);
        ring_test!(usize, 2, 4, 1_000, 8, multi_enqueue, multi_dequeue);
    }

    #[test]
    fn test_multi_producer_single_consumer() {
        ring_test!(usize, 2, 1, 1_000, 8, multi_enqueue, multi_dequeue);
        ring_test!(usize, 4, 2, 1_000, 8, multi_enqueue, multi_dequeue);
    }

    #[test]
    fn test_multi_producer_multi_consumer() {
        ring_test!(usize, 2, 2, 1_000, 8, multi_enqueue, multi_dequeue);
        ring_test!(usize, 4, 4, 1_000, 8, multi_enqueue, multi_dequeue);
        ring_test!(usize, 8, 8, 1_000, 8, multi_enqueue, multi_dequeue);
    }
}