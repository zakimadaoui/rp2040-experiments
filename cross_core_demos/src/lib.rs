#![no_std]

use rp2040_hal::pac;

pub struct CoreBridge;

impl CoreBridge {
    pub fn send_signal(irq: pac::Interrupt) {
        let sio = unsafe { &(*pac::SIO::PTR) };
        sio.fifo_wr.write(|wr| unsafe { wr.bits(irq as u32) });
    }

    pub fn read_signal() -> Option<pac::Interrupt> {
        let sio = unsafe { &(*pac::SIO::PTR) };
        if sio.fifo_st.read().vld().bit() {
            let irq = sio.fifo_rd.read().bits() as u16;
            let irq = unsafe { core::mem::transmute(irq) };
            Some(irq)
        } else {
            None
        }
    }
}

pub struct MessageQueue<T: Default + Copy, const DEPTH: usize> {
    buffer: [T; DEPTH],
    read_idx: usize,
    write_idx: usize,
}

impl<T: Default + Copy, const DEPTH: usize> MessageQueue<T, DEPTH> {
    pub fn new() -> Self {
        MessageQueue {
            buffer: [T::default(); DEPTH],
            read_idx: 0,
            write_idx: 0,
        }
    }

    pub fn push(&mut self, data: T) -> bool {
        if (self.write_idx + 1) % DEPTH != self.read_idx {
            self.buffer[self.write_idx] = data;
            self.write_idx = (self.write_idx + 1) % DEPTH;
            true
        } else {
            false
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.read_idx == self.write_idx {
            // Queue is empty
            None
        } else {
            let value = self.buffer[self.read_idx];
            self.read_idx = (self.read_idx + 1) % DEPTH;
            Some(value)
        }
    }
}

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
        let mut q = MessageQueue::<u32, 2>::new();
        assert_eq!(q.push(1), true);
        assert_eq!(q.push(2), true);
        assert_eq!(q.push(3), false); // TODO: fix this case
        // assert_eq!(q.pop(), Some(2));
        // assert_eq!(q.pop(), Some(3));
        // assert_eq!(q.pop(), None);
    }
}
