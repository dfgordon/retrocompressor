//! Ring buffer for LZ type compression windows
use num_traits::PrimInt;

pub struct RingBuffer<T: PrimInt> {
    buf: Vec<T>,
    pos: usize,
    n: usize
}

impl <T: PrimInt> RingBuffer<T> {
    pub fn create(fill: T,n: usize) -> Self {
        Self {
            buf: vec![fill;n],
            pos: 0,
            n
        }
    }
    /// get absolute position of cursor + offset
    pub fn get_pos(&self,offset: i64) -> usize {
        (self.pos as i64 + offset).rem_euclid(self.n as i64) as usize
    }
    /// set absolute position of cursor
    pub fn set_pos(&mut self,pos: usize) {
        self.pos = pos % self.n;
    }
    /// get value at absolute position, cursor does not move
    pub fn get_abs(&self,abs: usize) -> T {
        self.buf[abs % self.n]
    }
    /// set value at absolute position, cursor does not move
    pub fn set_abs(&mut self,abs: usize,val: T) {
        self.buf[abs % self.n] = val;
    }
    /// get value at cursor + offset
    pub fn get(&self,offset: i64) -> T {
        self.buf[(self.pos as i64 + offset).rem_euclid(self.n as i64) as usize]
    }
    /// set value at cursor + offset
    pub fn set(&mut self,offset: i64,val: T) {
        self.buf[(self.pos as i64 + offset).rem_euclid(self.n as i64) as usize] = val;
    }
    /// advance cursor by 1
    pub fn advance(&mut self) {
        self.pos = (self.pos + 1) % self.n;
    }
    /// retreat cursor by 1
    pub fn retreat(&mut self) {
        self.pos = (self.pos - 1) % self.n;
    }
    /// Distance to another position, assuming it is behind us.
    /// Correctly handles positions that are "ahead" in memory order.
    pub fn distance_behind(&self,other: usize) -> usize {
        (self.pos as i64 - other as i64).rem_euclid(self.n as i64) as usize
    }
}

#[test]
fn offset() {
    let mut ring: RingBuffer<u8> = RingBuffer::create(0,4);
    ring.set_pos(5);
    assert_eq!(ring.get_pos(0),1);
    assert_eq!(ring.get_pos(4),1);
    assert_eq!(ring.get_pos(3),0);
    assert_eq!(ring.get_pos(-4),1);
}

#[test]
fn distance() {
    // four positions 0 1 2 3
    // set position     ^       (wraps once)
    let mut ring: RingBuffer<u8> = RingBuffer::create(0,4);
    ring.set_pos(5);
    assert_eq!(ring.get_pos(0),1);
    assert_eq!(ring.distance_behind(0),1);
    assert_eq!(ring.distance_behind(1),0);
    assert_eq!(ring.distance_behind(3),2);
}
