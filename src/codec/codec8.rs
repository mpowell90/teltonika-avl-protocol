use crate::{AvlCodec, StackVec, avl::AvlDataRecord, crc16, error::AvlError};

pub const CODEC8_TYPE_ID: u8 = 0x08;

#[derive(Clone, Debug, PartialEq)]
pub struct Codec8Packet(pub StackVec<AvlDataRecord<u8>, 4>);

impl AvlCodec for Codec8Packet {
    fn size(&self) -> usize {
        4 + 4
            + 1
            + 1
            + self.0
                .iter()
                .map(|f| f.size())
                .sum::<usize>()
            + 1
            + 4
        // preamble + data_field_length + codec_id + data_1_count + data_2_count + avl_data_records + CRC16
    }

    fn encode(&self, buf: &mut [u8]) -> Result<usize, AvlError> {
        buf[0..4].copy_from_slice(&[0, 0, 0, 0]); // Preamble

        buf[4..8].copy_from_slice(&self.data_field_length().to_be_bytes());

        buf[8] = CODEC8_TYPE_ID;

        buf[9] = self.0.len() as u8; // data_1_count

        let mut offset = 10;

        for avl_data_record in &self.0 {
            offset += avl_data_record.encode(&mut buf[offset..])?;
        }

        buf[offset] = self.0.len() as u8; // data_2_count

        let data_field_length = self.data_field_length() as usize;

        // The CRC16 is encoded into 4 bytes even though it's a 2 byte value. The upper 2 bytes will always be 0.
        let crc16_value = crc16(&buf[8..(8 + data_field_length)]) as u32;

        buf[offset + 1..offset + 5].copy_from_slice(&crc16_value.to_be_bytes()); // CRC16

        Ok(offset + 5)
    }

    fn decode(buf: &[u8]) -> Result<(usize, Self), AvlError> {
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
        Ok((offset + 5, Self(avl_data_records)))
    }
}

impl Codec8Packet {
    pub const MIN_LENGTH: usize = 45;
    pub const MAX_LENGTH: usize = 1280;

    pub fn data_field_length(&self) -> u32 {
        self.0
            .iter()
            .map(|f| f.size())
            .sum::<usize>() as u32
            + 3 // codec_id + data_1_count + data_2_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::avl::*;

    fn sample_frame_with_io() -> AvlDataRecord<u8> {
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

    fn sample_frame_without_io() -> AvlDataRecord<u8> {
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
        let packet = Codec8Packet(StackVec::from_slice(&[sample_frame_without_io()]).unwrap());

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
        let packet = Codec8Packet(StackVec::from_slice(&[
            sample_frame_with_io(),
            sample_frame_without_io(),
        ])
        .unwrap());

        let mut encoded = [0_u8; 512];
        let encoded_len = packet.encode(&mut encoded).unwrap();

        let (bytes_decoded, decoded) = Codec8Packet::decode(&encoded[..encoded_len]).unwrap();
        assert_eq!(decoded.0.len(), 2);

        let mut re_encoded = [0_u8; 512];
        let re_encoded_len = decoded.encode(&mut re_encoded).unwrap();

        assert_eq!(encoded_len, re_encoded_len);
        assert_eq!(&encoded[..encoded_len], &re_encoded[..re_encoded_len]);
        assert_eq!(bytes_decoded, encoded_len);
    }

    #[test]
    fn decode_rejects_invalid_checksum() {
        let packet = Codec8Packet(StackVec::from_slice(&[sample_frame_with_io()]).unwrap());

        let mut encoded = [0_u8; 512];
        let encoded_len = packet.encode(&mut encoded).unwrap();

        // Corrupt checksum while keeping payload untouched.
        encoded[encoded_len - 1] ^= 0x01;

        let result = Codec8Packet::decode(&encoded[..encoded_len]);
        assert!(matches!(result, Err(AvlError::InvalidChecksum { .. })));
    }
}
