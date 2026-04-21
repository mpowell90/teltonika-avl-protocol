use crate::{AvlCodec, AvlIoElement, AvlIoId, error::AvlError};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AvlUdpRequest<T: AvlCodec> {
    pub packet_id: u16,
    pub avl_packet_header: AvlPacketHeader,
    pub avl_packet: T,
}

impl<T: AvlCodec> AvlUdpRequest<T> {
    pub fn size(&self) -> usize {
        5 + self.avl_packet_header.size() + self.avl_packet.size()
    }

    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, AvlError> {
        if buf.len() < self.size() {
            return Err(AvlError::InvalidFrame);
        }

        // the packet length does not include the 2 bytes for the packet length
        let packet_length =
            u16::try_from(3 + self.avl_packet_header.size() + self.avl_packet.size())
                .map_err(|_| AvlError::InvalidFrame)?;

        buf[0..2].copy_from_slice(&packet_length.to_be_bytes());
        buf[2..4].copy_from_slice(&self.packet_id.to_be_bytes());

        let mut offset = 4;

        offset += self.avl_packet_header.encode(&mut buf[offset..])?;

        offset += self.avl_packet.encode(&mut buf[offset..])?;

        Ok(offset)
    }

    pub fn decode(buf: &[u8]) -> Result<(usize, Self), AvlError> {
        if buf.len() < 20 {
            return Err(AvlError::InvalidFrame);
        }

        let packet_id = u16::from_be_bytes(buf[0..2].try_into().unwrap());

        let (header_size, avl_packet_header) = AvlPacketHeader::decode(&buf[2..])?;

        let (avl_packet_size, avl_packet) = T::decode(&buf[2 + header_size..])?;

        Ok((
            2 + header_size + avl_packet_size,
            Self {
                packet_id,
                avl_packet_header,
                avl_packet,
            },
        ))
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AvlUdpAckResponse {
    pub packet_id: u16,
    pub avl_packet_id: u8,
    pub accepted_avl_data_count: u8,
}

impl AvlUdpAckResponse {
    pub fn size(&self) -> usize {
        7 // packet length (2 bytes) + packet ID (2 bytes) + unused byte (1 byte) + AVL packet ID (1 byte) + accepted AVL data count (1 byte)
    }

    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, AvlError> {
        if buf.len() < self.size() {
            return Err(AvlError::InvalidFrame);
        }

        buf[0..2].copy_from_slice(&3u16.to_be_bytes()); // packet length is always 3 for the ACK response
        buf[2..4].copy_from_slice(&self.packet_id.to_be_bytes());
        buf[4] = 0x01; // not used byte - documentation states this is always set to 0x01
        buf[5] = self.avl_packet_id;
        buf[6] = self.accepted_avl_data_count;

        Ok(7)
    }

    pub fn decode(buf: &[u8]) -> Result<(usize, Self), AvlError> {
        if buf.len() < 7 {
            return Err(AvlError::InvalidFrame);
        }

        // TODO - do we really need to parse the packet length?

        let packet_id = u16::from_be_bytes(buf[2..4].try_into().unwrap());
        let avl_packet_id = buf[5];
        let accepted_avl_data_count = buf[6];

        Ok((
            7,
            Self {
                packet_id,
                avl_packet_id,
                accepted_avl_data_count,
            },
        ))
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AvlPacketHeader {
    pub packet_id: u8,
    pub imei: [u8; 15],
}

impl AvlPacketHeader {
    pub fn new(packet_id: u8, imei: [u8; 15]) -> Self {
        Self { packet_id, imei }
    }

    pub fn size(&self) -> usize {
        18
    }

    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, AvlError> {
        if buf.len() < 18 {
            return Err(AvlError::InvalidFrame);
        }
        buf[0] = self.packet_id;
        // imei length is always 0x000f
        buf[1] = 0; // imei length byte 1
        buf[2] = 0x0f; // imei length byte 2
        buf[3..18].copy_from_slice(&self.imei);
        Ok(18)
    }

    pub fn decode(buf: &[u8]) -> Result<(usize, Self), AvlError> {
        if buf.len() < 18 {
            return Err(AvlError::InvalidFrame);
        }

        let packet_id = buf[0];

        let mut imei = [0u8; 15];
        imei.copy_from_slice(&buf[3..18]);

        Ok((18, Self { packet_id, imei }))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AvlDataRecord<T> {
    pub timestamp: u64, // a difference, in milliseconds, between the current time and midnight, January, 1970 UTC (UNIX time).
    pub priority: Priority,
    pub gps_element: AvlGpsElement,
    pub io_element: T,
}

impl<T: AvlIoElement> AvlDataRecord<T> {
    pub fn size(&self) -> usize {
        AvlGpsElement::size() + self.io_element.size() + 9 // timestamp (8 bytes) + priority (1 byte)
    }

    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, AvlError> {
        buf[0..8].copy_from_slice(&self.timestamp.to_be_bytes());

        buf[8] = self.priority as u8;

        let mut offset = 9;

        offset += self.gps_element.encode(&mut buf[offset..])?;

        offset += self.io_element.encode(&mut buf[offset..])?;

        Ok(offset)
    }

    pub fn decode(buf: &[u8]) -> Result<(usize, Self), AvlError> {
        let timestamp = u64::from_be_bytes(buf[0..8].try_into().unwrap());
        let priority = Priority::try_from(buf[8])?;

        let mut offset = 9;
        let (gps_element_offset, gps_element) = AvlGpsElement::decode(&buf[9..])?;

        offset += gps_element_offset;

        let (io_element_offset, io_element) = T::decode(&buf[offset..])?;

        offset += io_element_offset;

        Ok((
            offset,
            Self {
                timestamp,
                priority,
                gps_element,
                io_element,
            },
        ))
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Priority {
    Low = 0,
    Medium = 1,
    High = 2,
}

impl TryFrom<u8> for Priority {
    type Error = AvlError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Priority::Low),
            1 => Ok(Priority::Medium),
            2 => Ok(Priority::High),
            value => Err(AvlError::InvalidPriority(value)),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Coordinate(pub f32);

impl Coordinate {
    pub const PRECISION: f32 = 10000000.0;

    pub fn encode(&self, buf: &mut [u8]) -> usize {
        let scaled = (self.0 * Self::PRECISION) as i32;
        buf[0..4].copy_from_slice(&scaled.to_be_bytes());
        4
    }

    pub fn decode(buf: &[u8]) -> Self {
        let bytes = i32::from_be_bytes(buf[0..4].try_into().unwrap());
        Self(bytes as f32 / Self::PRECISION)
    }
}

impl From<Coordinate> for f32 {
    fn from(coordinate: Coordinate) -> Self {
        coordinate.0
    }
}

impl From<f32> for Coordinate {
    fn from(value: f32) -> Self {
        Coordinate(value)
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AvlGpsElement {
    pub longitude: Coordinate, // east-west position, in degrees
    pub latitude: Coordinate,  // north-south position, in degrees
    pub altitude: i16,         // meters above sea level
    pub angle: u16,            // Degrees from north pole
    pub satellites: u8,        // number of visible satellites
    pub speed: u16,            // km/h
}

impl AvlGpsElement {
    pub fn size() -> usize {
        15
    }

    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, AvlError> {
        self.longitude.encode(&mut buf[0..4]);

        self.latitude.encode(&mut buf[4..8]);

        buf[8..10].copy_from_slice(&self.altitude.to_be_bytes());

        buf[10..12].copy_from_slice(&self.angle.to_be_bytes());

        buf[12] = self.satellites;

        buf[13..15].copy_from_slice(&self.speed.to_be_bytes());

        Ok(15)
    }

    pub fn decode(buf: &[u8]) -> Result<(usize, Self), AvlError> {
        Ok((
            15,
            Self {
                longitude: Coordinate::decode(&buf[0..4]),
                latitude: Coordinate::decode(&buf[4..8]),
                altitude: i16::from_be_bytes(buf[8..10].try_into().unwrap()),
                angle: u16::from_be_bytes(buf[10..12].try_into().unwrap()),
                satellites: buf[12],
                speed: u16::from_be_bytes(buf[13..15].try_into().unwrap()),
            },
        ))
    }
}

impl AvlIoId for u8 {
    fn size() -> usize {
        1
    }

    fn encode(&self, buf: &mut [u8]) -> Result<usize, AvlError> {
        buf[0] = *self;
        Ok(1)
    }

    fn decode(buf: &[u8]) -> Result<(usize, Self), AvlError> {
        Ok((1, buf[0]))
    }
}

impl AvlIoId for u16 {
    fn size() -> usize {
        2
    }

    fn encode(&self, buf: &mut [u8]) -> Result<usize, AvlError> {
        buf[0..2].copy_from_slice(&self.to_be_bytes());
        Ok(2)
    }

    fn decode(buf: &[u8]) -> Result<(usize, Self), AvlError> {
        Ok((2, u16::from_be_bytes(buf[0..2].try_into().unwrap())))
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AvlN1Element<T> {
    pub id: T,
    pub value: u8,
}

impl<T: AvlIoId> AvlN1Element<T> {
    pub fn size() -> usize {
        T::size() + 1
    }

    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, AvlError> {
        let offset = self.id.encode(buf)?;
        buf[offset] = self.value;
        Ok(offset + 1)
    }

    pub fn decode(buf: &[u8]) -> Result<(usize, Self), AvlError> {
        let (offset, id) = T::decode(buf)?;
        Ok((
            offset + 1,
            Self {
                id,
                value: buf[offset],
            },
        ))
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AvlN2Element<T> {
    pub id: T,
    pub value: u16,
}

impl<T: AvlIoId> AvlN2Element<T> {
    pub fn size() -> usize {
        T::size() + 2
    }

    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, AvlError> {
        let offset = self.id.encode(buf)?;
        buf[offset..offset + 2].copy_from_slice(&self.value.to_be_bytes());
        Ok(offset + 2)
    }

    pub fn decode(buf: &[u8]) -> Result<(usize, Self), AvlError> {
        let (offset, id) = T::decode(buf)?;
        Ok((
            offset + 2,
            Self {
                id,
                value: u16::from_be_bytes(buf[offset..offset + 2].try_into().unwrap()),
            },
        ))
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AvlN4Element<T> {
    pub id: T,
    pub value: u32,
}

impl<T: AvlIoId> AvlN4Element<T> {
    pub fn size() -> usize {
        T::size() + 4
    }

    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, AvlError> {
        let offset = self.id.encode(buf)?;
        buf[offset..offset + 4].copy_from_slice(&self.value.to_be_bytes());
        Ok(offset + 4)
    }

    pub fn decode(buf: &[u8]) -> Result<(usize, Self), AvlError> {
        let (offset, id) = T::decode(buf)?;
        Ok((
            offset + 4,
            Self {
                id,
                value: u32::from_be_bytes(buf[offset..offset + 4].try_into().unwrap()),
            },
        ))
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AvlN8Element<T> {
    pub id: T,
    pub value: u64,
}

impl<T: AvlIoId> AvlN8Element<T> {
    pub fn size() -> usize {
        T::size() + 8
    }

    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, AvlError> {
        let offset = self.id.encode(buf)?;
        buf[offset..offset + 8].copy_from_slice(&self.value.to_be_bytes());
        Ok(offset + 8)
    }

    pub fn decode(buf: &[u8]) -> Result<(usize, Self), AvlError> {
        let (offset, id) = T::decode(buf)?;
        Ok((
            offset + 8,
            Self {
                id,
                value: u64::from_be_bytes(buf[offset..offset + 8].try_into().unwrap()),
            },
        ))
    }
}
