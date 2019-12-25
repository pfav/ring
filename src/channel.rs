use std::cell::UnsafeCell;
use std::sync::Arc;
use std::sync::atomic::spin_loop_hint;
use std::fmt::Debug;

use crate::ring::Ring;

type ChannelRing<T> = Arc<UnsafeCell<Ring<T>>>;

#[repr(align(64))]
pub struct Channel<T> {
    ring: ChannelRing<T>,
}

impl<T: Debug> Channel<T> {
    #[inline]
    pub fn new(log2: usize) -> Self {
        let ring = Arc::new(UnsafeCell::new(Ring::new(log2)));
        Channel { ring }
    }
    
    #[inline]
    pub fn send(&self, v: T) {
        let mut value = v;
        loop {
            match self.enqueue(value) {
                Some(moved) => value = moved,
                None => break,
            }
            spin_loop_hint();
        }
    }

    #[inline]
    pub fn recv(&self) -> T {
        loop {
            if let Some(value) = self.dequeue() {
                break value;
            }
            spin_loop_hint();
        }
    }

    #[inline]
    pub fn clone(&self) -> Self {
        Channel{ ring: Arc::clone(&self.ring) }
    }

    #[inline]
    fn enqueue(&self, value: T) -> Option<T> {
        if Arc::strong_count(&self.ring) > 2 {
            unsafe { (*self.ring.get()).multi_enqueue(value) }
        } else {
            unsafe { (*self.ring.get()).single_enqueue(value) }
        }
    }

    #[inline]
    fn dequeue(&self) -> Option<T> {
        if Arc::strong_count(&self.ring) > 2 {
            unsafe { (*self.ring.get()).multi_dequeue() }
        } else {
            unsafe { (*self.ring.get()).single_dequeue() }
        }
    }
}

pub fn channel<T: Debug>(log2: usize) -> (Channel<T>, Channel<T>) {
    let chan = Channel::new(log2);
    (chan.clone(), chan)
}

unsafe impl<T> Sync for Channel<T> {}
unsafe impl<T> Send for Channel<T> {}

#[cfg(test)]
mod tests {
    // use super::*;

    #[test]
    fn constructor() {
    }
}