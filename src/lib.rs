pub mod codec8;
pub mod error;

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
