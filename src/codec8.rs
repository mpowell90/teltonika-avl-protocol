use crate::{crc16, error::AvlError, StackVec};

pub const CODEC8_TYPE_ID: u8 = 0x08;

#[derive(Clone, Debug, PartialEq)]
pub struct Codec8Packet {
    pub avl_data_records: StackVec<AvlDataRecord, 4>,
}

impl Codec8Packet {
    pub const MIN_LENGTH: usize = 45;
    pub const MAX_LENGTH: usize = 1280;

    pub fn data_field_length(&self) -> u32 {
        self.avl_data_records
            .iter()
            .map(|f| f.size())
            .sum::<usize>() as u32
            + 3 // codec_id + data_1_count + data_2_count
    }

    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, AvlError> {
        buf[0..4].copy_from_slice(&[0, 0, 0, 0]); // Preamble

        buf[4..8].copy_from_slice(&self.data_field_length().to_be_bytes());

        buf[8] = CODEC8_TYPE_ID;

        buf[9] = self.avl_data_records.len() as u8; // data_1_count

        let mut offset = 10;

        for avl_data_record in &self.avl_data_records {
            offset += avl_data_record.encode(&mut buf[offset..])?;
        }

        buf[offset] = self.avl_data_records.len() as u8; // data_2_count

        let data_field_length = self.data_field_length() as usize;

        // The CRC16 is encoded into 4 bytes even though it's a 2 byte value. The upper 2 bytes will always be 0.
        let crc16_value = crc16(&buf[8..(8 + data_field_length)]) as u32;

        buf[offset + 1..offset + 5].copy_from_slice(&crc16_value.to_be_bytes()); // CRC16

        Ok(offset + 5)
    }

    pub fn decode(buf: &[u8]) -> Result<(usize, Self), AvlError> {
        let data_field_length = u32::from_be_bytes(buf[4..8].try_into().unwrap());

        // The CRC16 is encoded into 4 bytes even though it's a 2 byte value. The upper 2 bytes will always be 0.
        let crc16_value = u16::from_be_bytes(
            buf[(10 + data_field_length as usize)..(10 + data_field_length as usize + 2)]
                .try_into()
                .unwrap(),
        );
        let computed_crc16_value = crc16(&buf[8..(8 + data_field_length as usize)]);

        if crc16_value != computed_crc16_value {
            return Err(AvlError::InvalidChecksum {
                expected: crc16_value,
                actual: computed_crc16_value,
            });
        }

        let data_1_count = buf[9];
        let data_2_count = buf[8 + data_field_length as usize - 1];

        if data_1_count != data_2_count {
            return Err(AvlError::InvalidDataCount {
                data_1_count,
                data_2_count,
            });
        }

        let mut avl_data_records = StackVec::new();
        let mut offset = 10;

        for _ in 0..data_1_count {
            let (bytes_read, avl_data_item) = AvlDataRecord::decode(&buf[offset..])?;
            avl_data_records.push(avl_data_item).unwrap();
            offset += bytes_read;
        }

        // final offset + 1 byte for data_2_count + 4 bytes for CRC16
        Ok((offset + 5, Self { avl_data_records }))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AvlDataRecord {
    pub timestamp: u64, // a difference, in milliseconds, between the current time and midnight, January, 1970 UTC (UNIX time).
    pub priority: Priority,
    pub gps_element: AvlGpsElement,
    pub event_io_id: u8,
    pub total_io_count: u8,
    pub n1_elements: StackVec<AvlN1Element, 16>,
    pub n2_elements: StackVec<AvlN2Element, 16>,
    pub n4_elements: StackVec<AvlN4Element, 16>,
    pub n8_elements: StackVec<AvlN8Element, 16>,
}

impl AvlDataRecord {
    pub fn size(&self) -> usize {
        let gps_element_size = 15; // 15 bytes for GPS element
        let io_elements_size: usize = self.n1_elements.len() * 2
            + self.n2_elements.len() * 3
            + self.n4_elements.len() * 5
            + self.n8_elements.len() * 9;

        // timestamp + priority + GPS element + event IO ID + total IO count + N1/N2/N4/N8 group counts + IO elements
        8 + 1 + gps_element_size + 1 + 1 + 4 + io_elements_size
    }

    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, AvlError> {
        buf[0..8].copy_from_slice(&self.timestamp.to_be_bytes());

        buf[8] = self.priority as u8;

        let mut offset = 9 + self.gps_element.encode(&mut buf[9..])?;

        // IO Element
        buf[offset] = self.event_io_id;
        offset += 1;

        // Total IO count
        buf[offset] = self.n1_elements.len() as u8
            + self.n2_elements.len() as u8
            + self.n4_elements.len() as u8
            + self.n8_elements.len() as u8;
        offset += 1;

        let n1_count = self.n1_elements.len();
        buf[offset] = n1_count as u8;
        offset += 1;

        if n1_count > 0 {
            for elem in &self.n1_elements {
                offset += elem.encode(&mut buf[offset..])?;
            }
        }

        let n2_count = self.n2_elements.len();
        buf[offset] = n2_count as u8;
        offset += 1;

        if n2_count > 0 {
            for elem in &self.n2_elements {
                offset += elem.encode(&mut buf[offset..])?;
            }
        }

        let n4_count = self.n4_elements.len();
        buf[offset] = n4_count as u8;
        offset += 1;

        if n4_count > 0 {
            for elem in &self.n4_elements {
                offset += elem.encode(&mut buf[offset..])?;
            }
        }

        let n8_count = self.n8_elements.len();
        buf[offset] = n8_count as u8;
        offset += 1;

        if n8_count > 0 {
            for elem in &self.n8_elements {
                offset += elem.encode(&mut buf[offset..])?;
            }
        }

        Ok(offset)
    }

    pub fn decode(buf: &[u8]) -> Result<(usize, Self), AvlError> {
        let timestamp = u64::from_be_bytes(buf[0..8].try_into().unwrap());
        let priority = Priority::try_from(buf[8])?;
        let gps_element = AvlGpsElement::decode(&buf[9..])?;

        let mut offset = 9 + 15; // 15 bytes for GPS element

        // IO Element
        let event_io_id = buf[offset];
        offset += 1;

        let total_io_count = buf[offset];
        offset += 1;

        let n1_io_count = buf[offset];
        offset += 1;

        let mut n1_elements = StackVec::new();

        if n1_io_count > 0 {
            let chunk_size: usize = 2;
            let stride = n1_io_count as usize * chunk_size;

            for chunk in buf[offset..(offset + stride)].chunks(chunk_size) {
                n1_elements
                    .push(AvlN1Element::decode(chunk).unwrap())
                    .unwrap();
            }

            offset += stride;
        }

        let n2_io_count = buf[offset];
        offset += 1;

        let mut n2_elements = StackVec::new();

        if n2_io_count > 0 {
            let chunk_size: usize = 3;
            let stride = n2_io_count as usize * chunk_size;

            for chunk in buf[offset..(offset + stride)].chunks(chunk_size) {
                n2_elements
                    .push(AvlN2Element::decode(chunk).unwrap())
                    .unwrap();
            }

            offset += stride;
        }

        let n4_io_count = buf[offset];
        offset += 1;

        let mut n4_elements = StackVec::new();

        if n4_io_count > 0 {
            let chunk_size: usize = 5;
            let stride = n4_io_count as usize * chunk_size;

            for chunk in buf[offset..(offset + stride)].chunks(chunk_size) {
                n4_elements
                    .push(AvlN4Element::decode(chunk).unwrap())
                    .unwrap();
            }

            offset += stride;
        }

        let n8_io_count = buf[offset];
        offset += 1;

        let mut n8_elements = StackVec::new();

        if n8_io_count > 0 {
            let chunk_size: usize = 9;
            let stride = n8_io_count as usize * chunk_size;

            for chunk in buf[offset..(offset + stride)].chunks(chunk_size) {
                n8_elements
                    .push(AvlN8Element::decode(chunk).unwrap())
                    .unwrap();
            }

            offset += stride;
        }

        Ok((
            offset,
            Self {
                timestamp,
                priority,
                gps_element,
                event_io_id,
                total_io_count,
                n1_elements,
                n2_elements,
                n4_elements,
                n8_elements,
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
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, AvlError> {
        self.longitude.encode(&mut buf[0..4]);

        self.latitude.encode(&mut buf[4..8]);

        buf[8..10].copy_from_slice(&self.altitude.to_be_bytes());

        buf[10..12].copy_from_slice(&self.angle.to_be_bytes());

        buf[12] = self.satellites;

        buf[13..15].copy_from_slice(&self.speed.to_be_bytes());

        Ok(15)
    }

    pub fn decode(buf: &[u8]) -> Result<Self, AvlError> {
        Ok(Self {
            longitude: Coordinate::decode(&buf[0..4]),
            latitude: Coordinate::decode(&buf[4..8]),
            altitude: i16::from_be_bytes(buf[8..10].try_into().unwrap()),
            angle: u16::from_be_bytes(buf[10..12].try_into().unwrap()),
            satellites: buf[12],
            speed: u16::from_be_bytes(buf[13..15].try_into().unwrap()),
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AvlN1Element {
    pub id: u8,
    pub value: u8,
}

impl AvlN1Element {
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, AvlError> {
        buf[0] = self.id;
        buf[1] = self.value;
        Ok(2)
    }

    pub fn decode(buf: &[u8]) -> Result<Self, AvlError> {
        Ok(Self {
            id: buf[0],
            value: buf[1],
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AvlN2Element {
    pub id: u8,
    pub value: u16,
}

impl AvlN2Element {
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, AvlError> {
        buf[0] = self.id;
        buf[1..3].copy_from_slice(&self.value.to_be_bytes());
        Ok(3)
    }

    pub fn decode(buf: &[u8]) -> Result<Self, AvlError> {
        Ok(Self {
            id: buf[0],
            value: u16::from_be_bytes(buf[1..3].try_into().unwrap()),
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AvlN4Element {
    pub id: u8,
    pub value: u32,
}

impl AvlN4Element {
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, AvlError> {
        buf[0] = self.id;
        buf[1..5].copy_from_slice(&self.value.to_be_bytes());
        Ok(5)
    }

    pub fn decode(buf: &[u8]) -> Result<Self, AvlError> {
        Ok(Self {
            id: buf[0],
            value: u32::from_be_bytes(buf[1..5].try_into().unwrap()),
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AvlN8Element {
    pub id: u8,
    pub value: u64,
}

impl AvlN8Element {
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, AvlError> {
        buf[0] = self.id;
        buf[1..9].copy_from_slice(&self.value.to_be_bytes());
        Ok(9)
    }

    pub fn decode(buf: &[u8]) -> Result<Self, AvlError> {
        Ok(Self {
            id: buf[0],
            value: u64::from_be_bytes(buf[1..9].try_into().unwrap()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_frame_with_io() -> AvlDataRecord {
        // example from https://wiki.teltonika-gps.com/view/Teltonika_AVL_Protocols#Codec_8 AVL Data Packet example section
        AvlDataRecord {
            timestamp: 0x000000016b40d8ea30,
            priority: Priority::Medium,
            gps_element: AvlGpsElement {
                longitude: Coordinate(0.0),
                latitude: Coordinate(0.0),
                altitude: 0,
                angle: 0,
                satellites: 0,
                speed: 0,
            },
            event_io_id: 1,
            total_io_count: 4,
            n1_elements: StackVec::from_slice(&[AvlN1Element {
                id: 0x15,
                value: 0x03,
            }])
            .unwrap(),
            n2_elements: StackVec::from_slice(&[AvlN2Element {
                id: 0x42,
                value: 0x5e0f,
            }])
            .unwrap(),
            n4_elements: StackVec::from_slice(&[AvlN4Element {
                id: 0xf1,
                value: 0x0000601a,
            }])
            .unwrap(),
            n8_elements: StackVec::from_slice(&[AvlN8Element {
                id: 0x4e,
                value: 0x0,
            }])
            .unwrap(),
        }
    }

    fn sample_frame_without_io() -> AvlDataRecord {
        AvlDataRecord {
            timestamp: 0x000000016b40d8ea30,
            priority: Priority::Low,
            gps_element: AvlGpsElement {
                longitude: Coordinate(10.1234),
                latitude: Coordinate(-33.9230),
                altitude: 5,
                angle: 15,
                satellites: 5,
                speed: 22,
            },
            event_io_id: 2,
            total_io_count: 0,
            n1_elements: StackVec::new(),
            n2_elements: StackVec::new(),
            n4_elements: StackVec::new(),
            n8_elements: StackVec::new(),
        }
    }

    #[test]
    fn encodes_packet_header_and_crc_correctly() {
        let packet = Codec8Packet {
            avl_data_records: StackVec::from_slice(&[sample_frame_without_io()]).unwrap(),
        };

        let mut buf = [0_u8; 256];
        let bytes_written = packet.encode(&mut buf).unwrap();

        assert_eq!(bytes_written, 45);
        assert_eq!(&buf[0..4], &[0, 0, 0, 0]);
        assert_eq!(u32::from_be_bytes(buf[4..8].try_into().unwrap()), 33);
        assert_eq!(buf[8], CODEC8_TYPE_ID);
        assert_eq!(buf[9], 1);
        assert_eq!(buf[40], 1);

        let expected_crc = crc16(&buf[8..41]);
        let actual_crc = u16::from_be_bytes(buf[43..45].try_into().unwrap());
        assert_eq!(actual_crc, expected_crc);
    }

    #[test]
    fn round_trip_encode_decode_preserves_payload() {
        let packet = Codec8Packet {
            avl_data_records: StackVec::from_slice(&[
                sample_frame_with_io(),
                sample_frame_without_io(),
            ])
            .unwrap(),
        };

        let mut encoded = [0_u8; 512];
        let encoded_len = packet.encode(&mut encoded).unwrap();

        let (bytes_decoded, decoded) = Codec8Packet::decode(&encoded[..encoded_len]).unwrap();
        assert_eq!(decoded.avl_data_records.len(), 2);

        let mut re_encoded = [0_u8; 512];
        let re_encoded_len = decoded.encode(&mut re_encoded).unwrap();

        assert_eq!(encoded_len, re_encoded_len);
        assert_eq!(&encoded[..encoded_len], &re_encoded[..re_encoded_len]);
        assert_eq!(bytes_decoded, encoded_len);
    }

    #[test]
    fn decode_rejects_invalid_checksum() {
        let packet = Codec8Packet {
            avl_data_records: StackVec::from_slice(&[sample_frame_with_io()]).unwrap(),
        };

        let mut encoded = [0_u8; 512];
        let encoded_len = packet.encode(&mut encoded).unwrap();

        // Corrupt checksum while keeping payload untouched.
        encoded[encoded_len - 1] ^= 0x01;

        let result = Codec8Packet::decode(&encoded[..encoded_len]);
        assert!(matches!(result, Err(AvlError::InvalidChecksum { .. })));
    }
}
