use std::mem;
use std::ptr;
use std::marker::PhantomData;
use std::fmt::{self, Debug};

#[repr(align(64))]
pub struct Buffer<T> {
    inner: *mut T,
    pub size: usize,
    _marker: PhantomData<T>,
}

#[allow(dead_code)]
impl<T> Buffer<T> {
    
    #[inline]
    pub fn new(size : usize) -> Self {
        let inner = {
            let mut v = Vec::<T>::with_capacity(size);
            let buffer = v.as_mut_ptr();
            mem::forget(v);
            buffer
        };
        
        Buffer { 
            inner,
            size,
            _marker: PhantomData,
        }
    }

    #[inline]
    pub fn as_ptr(&self, index: usize) -> *const T {
        assert!(index < self.size);
        
        unsafe { self.inner.offset(index as isize) as *const T }
    }

    #[inline]
    pub fn as_mut_ptr(&mut self, index: usize) -> *mut T {
        assert!(index < self.size as usize);
        
        unsafe { self.inner.offset(index as isize) }
    }

    #[inline]
    pub fn write(&mut self, index: usize, value: T) {        
        unsafe { ptr::write_unaligned(self.as_mut_ptr(index), value) }
    }

    #[inline]
    pub fn read(&self, index: usize) -> T {
        unsafe { ptr::read_unaligned(self.as_ptr(index)) }
    }

    #[inline]
    pub fn at(&self, index: usize) -> &T {
        unsafe { &*self.as_ptr(index) }
    }

    #[inline]
    pub fn at_mut(&mut self, index: usize) -> &mut T {
        unsafe { &mut *self.as_mut_ptr(index) }
    }

    #[inline]
    pub fn drop_at(&mut self, index: usize) {
        unsafe { self.as_mut_ptr(index).drop_in_place() }
    }
}

impl<T: Debug> Debug for Buffer<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[ ")?;
        for i in 0..self.size {
            write!(f, "{:?} ", self.read(i as usize))?;
        }
        write!(f, " ]")
    }
}

impl<T> Drop for Buffer<T> {
    #[inline(always)]
    fn drop(&mut self) {
        let v = unsafe { Vec::from_raw_parts(self.inner, 0, self.size as usize) };

        drop(v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor() {
        let buf : Buffer<u8> = Buffer::new(8);
        assert_eq!(buf.size, 8);
    }

    #[test]
    fn read_write() {
        let mut buf : Buffer<u8> = Buffer::new(8);
        buf.write(0, 127);
        buf.write(7, 128);

        assert_eq!(buf.read(0), 127);
        assert_eq!(buf.read(7), 128);
    }

    #[test]
    #[should_panic]
    fn test_overflow() {
        let mut buf : Buffer<u8> = Buffer::new(8);
        buf.write(8, 10);
    }

    static mut N : usize = 0;

    struct S { num: usize }
    impl S { 
        fn new() -> Self {
            let num = unsafe {
                N += 1;
                N
            };
            S{ num }
        }
    }
    impl Drop for S {
        fn drop(&mut self) {
            unsafe { N -= 1 };
        }
    }

    #[test]
    fn test_drop() {
        let s = S::new();
        assert_eq!(unsafe { N }, 1);
        drop(s);
        assert_eq!(unsafe { N }, 0);

        let mut buf : Buffer<S> = Buffer::new(9);
        buf.write(0, S::new());
        assert_eq!(buf.at(0).num, 1);

        buf.write(1, S::new());
        assert_eq!(buf.at(1).num, 2);

        assert_eq!(unsafe { N }, 2);

        buf.drop_at(0);
        assert_eq!(unsafe { N }, 1);

        buf.drop_at(1);
        assert_eq!(unsafe { N }, 0);
    }
}