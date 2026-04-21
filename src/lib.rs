#![cfg_attr(not(test), no_std)]

pub mod codec;
pub mod avl;
pub mod error;

pub use heapless::Vec as StackVec;

// https://wiki.teltonika-gps.com/view/Codec#CRC-16
pub fn crc16(msg: &[u8]) -> u16 {
    let mut crc: u16 = 0x0;

    for byte in msg.iter() {
        crc ^= *byte as u16;

        for _ in 0..=7 {
            let carry = crc & 1;

            crc >>= 1;

            if carry == 1 {
                crc ^= 0xa001
            }
        }
    }

    crc
}

pub trait AvlCodec {
    fn size(&self) -> usize;

    fn encode(&self, buf: &mut [u8]) -> Result<usize, error::AvlError>;

    fn decode(buf: &[u8]) -> Result<(usize, Self), error::AvlError>
    where
        Self: Sized;
}
