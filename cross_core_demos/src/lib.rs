#![no_std]

use core::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    sync::atomic::{AtomicUsize, Ordering},
};

use rp2040_hal::pac;

#[allow(non_snake_case)]
pub mod CrossCore {
    use super::*;
    pub fn pend_irq(irq: pac::Interrupt, _core_id: u32) {
        let sio = unsafe { &(*pac::SIO::PTR) };
        sio.fifo_wr.write(|wr| unsafe { wr.bits(irq as u32) });
    }

    pub fn get_pended_irq() -> Option<pac::Interrupt> {
        let sio = unsafe { &(*pac::SIO::PTR) };
        if sio.fifo_st.read().vld().bit() {
            let irq = sio.fifo_rd.read().bits() as u16;
            // implementation must guarantee that the only messages passed in the fifo are of pac::Interrupt type.
            let irq = unsafe { core::mem::transmute(irq) };
            Some(irq)
        } else {
            None
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct FullQueueError;

pub struct MessageQueue<T: Default + Copy, const DEPTH: usize> {
    buffer: UnsafeCell<[MaybeUninit<T>; DEPTH]>,
    read_idx: AtomicUsize,
    write_idx: AtomicUsize,
}

impl<T: Default + Copy, const DEPTH: usize> MessageQueue<T, DEPTH> {
    #[inline(always)]
    pub fn new() -> Self {
        MessageQueue {
            buffer: UnsafeCell::from([MaybeUninit::zeroed(); DEPTH]),
            read_idx: 0.into(),
            write_idx: 0.into(),
        }
    }

    pub fn push(&self, data: T) -> Result<(), FullQueueError> {
        let r = self.read_idx.load(Ordering::Relaxed) % DEPTH;
        let w = self.write_idx.load(Ordering::Acquire) % DEPTH;

        if (w + 1) % DEPTH != r {
            unsafe { (self.buffer.get() as *mut T).add(w).write(data) };
            self.write_idx.store(w + 1, Ordering::Release);
            Ok(())
        } else {
            Err(FullQueueError)
        }
    }

    pub fn pop(&self) -> Option<T> {
        let w = self.write_idx.load(Ordering::Relaxed) % DEPTH;
        let r = self.read_idx.load(Ordering::Acquire) % DEPTH;
        if r == w {
            None
        } else {
            let data = unsafe { (self.buffer.get() as *mut T).add(r % DEPTH).read() };
            self.read_idx.store(r + 1, Ordering::Release);
            Some(data)
        }
    }
}

unsafe impl<T: Default + Copy, const DEPTH: usize> Sync for MessageQueue<T, DEPTH> {}

impl<T: Default + Copy, const DEPTH: usize> Default for MessageQueue<T, DEPTH> {
    fn default() -> Self {
        Self::new()
    }
}

// tests

#[cfg(test)]
mod tests {
    use crate::MessageQueue;

    #[test]
    fn test_queue1() {
        let q = MessageQueue::<u32, 3>::new();
        assert!(q.push(1).is_ok());
        assert!(q.push(2).is_ok());
        assert!(q.push(3).is_err()); // In a ring buffer implementation with a read/write index, one element is always unused
        assert_eq!(q.pop(), Some(1));
        assert_eq!(q.pop(), Some(2));
        assert_eq!(q.pop(), None);
        assert!(q.push(4).is_ok());
        assert!(q.push(5).is_ok());
        assert!(q.push(6).is_err());
        assert_eq!(q.pop(), Some(4));
        assert_eq!(q.pop(), Some(5));
        assert!(q.push(7).is_ok());
        assert_eq!(q.pop(), Some(7));
        assert_eq!(q.pop(), None);
        assert!(q.push(8).is_ok());
        assert!(q.push(9).is_ok());
        assert_eq!(q.pop(), Some(8));
        assert_eq!(q.pop(), Some(9));
        assert_eq!(q.pop(), None);
    }
}
