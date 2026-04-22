use crate::{
    AvlCodec, AvlIoElement, StackVec,
    avl::{AvlDataRecord, AvlN1Element, AvlN2Element, AvlN4Element, AvlN8Element},
    crc16,
    error::AvlError,
};

pub const CODEC8_TYPE_ID: u8 = 0x08;

#[derive(Clone, Debug, PartialEq)]
pub struct Codec8Packet(pub StackVec<AvlDataRecord<Codec8IoElement>, 4>);

impl AvlCodec for Codec8Packet {
    fn size(&self) -> usize {
        self.0.iter().map(|f| f.size()).sum::<usize>() + 15 // preamble + data_field_length + codec_id + data_1_count + data_2_count + avl_data_records + CRC16
    }

    fn encode(&self, buf: &mut [u8]) -> Result<usize, AvlError> {
        buf[0..4].copy_from_slice(&[0, 0, 0, 0]); // Preamble

        let data_field_length = self.size() - 12; // without preamble and data_field_length and CRC16

        buf[4..8].copy_from_slice(&(data_field_length as u32).to_be_bytes());

        buf[8] = CODEC8_TYPE_ID;

        buf[9] = self.0.len() as u8; // data_1_count

        let mut offset = 10;

        for avl_data_record in &self.0 {
            offset += avl_data_record.encode(&mut buf[offset..])?;
        }

        buf[offset] = self.0.len() as u8; // data_2_count

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

#[derive(Clone, Debug, PartialEq)]
pub struct Codec8IoElement {
    pub event_io_id: u8,
    pub total_io_count: u8,
    pub n1_elements: StackVec<AvlN1Element<u8>, 16>,
    pub n2_elements: StackVec<AvlN2Element<u8>, 16>,
    pub n4_elements: StackVec<AvlN4Element<u8>, 16>,
    pub n8_elements: StackVec<AvlN8Element<u8>, 16>,
}

impl AvlIoElement for Codec8IoElement {
    fn size(&self) -> usize {
        6 + (self.n1_elements.len() * AvlN1Element::<u8>::size())
            + (self.n2_elements.len() * AvlN2Element::<u8>::size())
            + (self.n4_elements.len() * AvlN4Element::<u8>::size())
            + (self.n8_elements.len() * AvlN8Element::<u8>::size())
    }

    fn encode(&self, buf: &mut [u8]) -> Result<usize, AvlError> {
        buf[0] = self.event_io_id;
        buf[1] = self.total_io_count;

        let mut offset = 0;

        buf[offset] = self.event_io_id;
        offset += 1;

        buf[offset] = self.total_io_count;
        offset += 1;

        let n1_count = self.n1_elements.len();
        buf[offset] = n1_count as u8;
        offset += 1;

        for elem in &self.n1_elements {
            offset += elem.encode(&mut buf[offset..])?;
        }

        let n2_count = self.n2_elements.len();
        buf[offset] = n2_count as u8;
        offset += 1;

        for elem in &self.n2_elements {
            offset += elem.encode(&mut buf[offset..])?;
        }

        let n4_count = self.n4_elements.len();
        buf[offset] = n4_count as u8;
        offset += 1;

        for elem in &self.n4_elements {
            offset += elem.encode(&mut buf[offset..])?;
        }

        let n8_count = self.n8_elements.len();
        buf[offset] = n8_count as u8;
        offset += 1;

        for elem in &self.n8_elements {
            offset += elem.encode(&mut buf[offset..])?;
        }

        Ok(offset)
    }

    fn decode(buf: &[u8]) -> Result<(usize, Self), AvlError> {
        let mut offset = 0;

        let event_io_id = buf[offset];
        offset += 1;

        let total_io_count = buf[offset];
        offset += 1;

        let n1_io_count = buf[offset];
        offset += 1;

        let mut n1_elements = StackVec::new();

        for _ in 0..n1_io_count {
            let (bytes_read, n1_element) = AvlN1Element::decode(&buf[offset..])?;
            n1_elements.push(n1_element).unwrap();
            offset += bytes_read;
        }

        let n2_io_count = buf[offset];
        offset += 1;

        let mut n2_elements = StackVec::new();

        for _ in 0..n2_io_count {
            let (bytes_read, n2_element) = AvlN2Element::decode(&buf[offset..])?;
            n2_elements.push(n2_element).unwrap();
            offset += bytes_read;
        }

        let n4_io_count = buf[offset];
        offset += 1;

        let mut n4_elements = StackVec::new();

        for _ in 0..n4_io_count {
            let (bytes_read, n4_element) = AvlN4Element::decode(&buf[offset..])?;
            n4_elements.push(n4_element).unwrap();
            offset += bytes_read;
        }

        let n8_io_count = buf[offset];
        offset += 1;

        let mut n8_elements = StackVec::new();

        for _ in 0..n8_io_count {
            let (bytes_read, n8_element) = AvlN8Element::decode(&buf[offset..])?;
            n8_elements.push(n8_element).unwrap();
            offset += bytes_read;
        }

        Ok((
            offset,
            Self {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::avl::*;

    fn example1() -> AvlDataRecord<Codec8IoElement> {
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
            io_element: Codec8IoElement {
                event_io_id: 1,
                total_io_count: 5,
                n1_elements: StackVec::from_slice(&[
                    AvlN1Element {
                        id: 0x15,
                        value: 0x03,
                    },
                    AvlN1Element {
                        id: 0x01,
                        value: 0x01,
                    },
                ])
                .unwrap(),
                n2_elements: StackVec::from_slice(&[AvlN2Element {
                    id: 0x42,
                    value: 0x5e0f,
                }])
                .unwrap(),
                n4_elements: StackVec::from_slice(&[AvlN4Element {
                    id: 0xf1,
                    value: 0x601a,
                }])
                .unwrap(),
                n8_elements: StackVec::from_slice(&[AvlN8Element {
                    id: 0x4e,
                    value: 0x00,
                }])
                .unwrap(),
            },
        }
    }

    #[test]
    fn should_encode_decode_example1_codec8_packet() {
        let mut buf = [0_u8; 256];

        let packet = Codec8Packet(StackVec::from_slice(&[example1()]).unwrap());
        let bytes_encoded = packet.encode(&mut buf).unwrap();

        assert_eq!(
            &buf[0..bytes_encoded],
            &[
                0, 0, 0, 0, // Preamble
                0, 0, 0, 0x36, // Data field length
                8,    // Codec ID
                1,    // Data 1 count
                0x00, 0x00, 0x01, 0x6b, 0x40, 0xd8, 0xea, 0x30, // Timestamp
                1,    // Priority
                0, 0, 0, 0, // GPS element - Longitude
                0, 0, 0, 0, // GPS element - Latitude
                0, 0, // GPS element - Altitude
                0, 0, // GPS element - Angle
                0, // GPS element - Satellites
                0, 0,    // GPS element - Speed
                1,    // Event IO ID
                5,    // Total IO count
                2,    // N1 elements count
                0x15, // N1 element 1 ID
                3,    // N1 element 1 value
                1,    // N1 element 2 ID
                1,    // N1 element 2 value
                1,    // N2 element count
                0x42, // N2 element ID
                0x5e, 0x0f, // N2 element value
                1,    // N4 element count
                0xf1, // N4 element ID
                0x00, 0x00, 0x60, 0x1a, // N4 element value
                1,    // N8 element count
                0x4e, // N8 element ID
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // N8 element value
                1,    // Data 2 count
                0, 0, 0xc7, 0xcf // CRC16
            ]
        );

        let (bytes_decoded, decoded_packet) = Codec8Packet::decode(&buf[..bytes_encoded]).unwrap();
        assert_eq!(bytes_encoded, bytes_decoded);

        assert_eq!(packet, decoded_packet);
    }

    fn example2() -> AvlDataRecord<Codec8IoElement> {
        // example from https://wiki.teltonika-gps.com/view/Teltonika_AVL_Protocols#Codec_8 AVL Data Packet example section
        AvlDataRecord {
            timestamp: 0x000000016b40d9ad80,
            priority: Priority::Medium,
            gps_element: AvlGpsElement {
                longitude: Coordinate(0.0),
                latitude: Coordinate(0.0),
                altitude: 0,
                angle: 0,
                satellites: 0,
                speed: 0,
            },
            io_element: Codec8IoElement {
                event_io_id: 1,
                total_io_count: 3,
                n1_elements: StackVec::from_slice(&[
                    AvlN1Element {
                        id: 0x15,
                        value: 0x03,
                    },
                    AvlN1Element {
                        id: 0x01,
                        value: 0x01,
                    },
                ])
                .unwrap(),
                n2_elements: StackVec::from_slice(&[AvlN2Element {
                    id: 0x42,
                    value: 0x5e10,
                }])
                .unwrap(),
                n4_elements: StackVec::new(),
                n8_elements: StackVec::new(),
            },
        }
    }

    #[test]
    fn should_encode_decode_example2_codec8_packet() {
        let mut buf = [0_u8; 256];

        let packet = Codec8Packet(StackVec::from_slice(&[example2()]).unwrap());
        let bytes_encoded = packet.encode(&mut buf).unwrap();

        assert_eq!(
            &buf[0..bytes_encoded],
            &[
                0, 0, 0, 0, // Preamble
                0, 0, 0, 0x28, // Data field length
                8,    // Codec ID
                1,    // Data 1 count
                0x00, 0x00, 0x01, 0x6b, 0x40, 0xd9, 0xad, 0x80, // Timestamp
                1,    // Priority
                0, 0, 0, 0, // GPS element - Longitude
                0, 0, 0, 0, // GPS element - Latitude
                0, 0, // GPS element - Altitude
                0, 0, // GPS element - Angle
                0, // GPS element - Satellites
                0, 0,    // GPS element - Speed
                1,    // Event IO ID
                3,    // Total IO count
                2,    // N1 elements count
                0x15, // N1 element 1 ID
                3,    // N1 element 1 value
                1,    // N1 element 2 ID
                1,    // N1 element 2 value
                1,    // N2 element count
                0x42, // N2 element ID
                0x5e, 0x10, // N2 element value
                0,    // N4 element count
                0,    // N8 element count
                1,    // Data 2 count
                0, 0, 0xf2, 0x2a // CRC16
            ]
        );

        let (bytes_decoded, decoded_packet) = Codec8Packet::decode(&buf[..bytes_encoded]).unwrap();
        assert_eq!(bytes_encoded, bytes_decoded);

        assert_eq!(packet, decoded_packet);
    }

    fn example3() -> Codec8Packet {
        // example from https://wiki.teltonika-gps.com/view/Teltonika_AVL_Protocols#Codec_8 AVL Data Packet example section
        Codec8Packet(
            StackVec::from_slice(&[
                AvlDataRecord {
                    timestamp: 0x000000016b40d57b48,
                    priority: Priority::Medium,
                    gps_element: AvlGpsElement {
                        longitude: Coordinate(0.0),
                        latitude: Coordinate(0.0),
                        altitude: 0,
                        angle: 0,
                        satellites: 0,
                        speed: 0,
                    },
                    io_element: Codec8IoElement {
                        event_io_id: 1,
                        total_io_count: 1,
                        n1_elements: StackVec::from_slice(&[AvlN1Element {
                            id: 0x01,
                            value: 0x00,
                        }])
                        .unwrap(),
                        n2_elements: StackVec::new(),
                        n4_elements: StackVec::new(),
                        n8_elements: StackVec::new(),
                    },
                },
                AvlDataRecord {
                    timestamp: 0x000000016b40d5c198,
                    priority: Priority::Medium,
                    gps_element: AvlGpsElement {
                        longitude: Coordinate(0.0),
                        latitude: Coordinate(0.0),
                        altitude: 0,
                        angle: 0,
                        satellites: 0,
                        speed: 0,
                    },
                    io_element: Codec8IoElement {
                        event_io_id: 1,
                        total_io_count: 1,
                        n1_elements: StackVec::from_slice(&[AvlN1Element {
                            id: 0x01,
                            value: 0x01,
                        }])
                        .unwrap(),
                        n2_elements: StackVec::new(),
                        n4_elements: StackVec::new(),
                        n8_elements: StackVec::new(),
                    },
                },
            ])
            .unwrap(),
        )
    }

    #[test]
    fn should_encode_decode_example3_codec8_packet() {
        let mut buf = [0_u8; 256];

        let packet = example3();
        let bytes_encoded = packet.encode(&mut buf).unwrap();

        assert_eq!(
            &buf[0..bytes_encoded],
            &[
                0, 0, 0, 0, // Preamble
                0, 0, 0, 0x43, // Data field length
                8,    // Codec ID
                2,    // Data 1 count
                // Record 1
                0x00, 0x00, 0x01, 0x6b, 0x40, 0xd5, 0x7b, 0x48, // Timestamp
                1,    // Priority
                0, 0, 0, 0, // GPS element - Longitude
                0, 0, 0, 0, // GPS element - Latitude
                0, 0, // GPS element - Altitude
                0, 0, // GPS element - Angle
                0, // GPS element - Satellites
                0, 0, // GPS element - Speed
                1, // Event IO ID
                1, // Total IO count
                1, // N1 elements count
                1, // N1 element 1 ID
                0, // N1 element 1 value
                0, // N2 element count
                0, // N4 element count
                0, // N8 element count
                // Record 2
                0x00, 0x00, 0x01, 0x6b, 0x40, 0xd5, 0xc1, 0x98, // Timestamp
                1,    // Priority
                0, 0, 0, 0, // GPS element - Longitude
                0, 0, 0, 0, // GPS element - Latitude
                0, 0, // GPS element - Altitude
                0, 0, // GPS element - Angle
                0, // GPS element - Satellites
                0, 0, // GPS element - Speed
                1, // Event IO ID
                1, // Total IO count
                1, // N1 elements count
                1, // N1 element 1 ID
                1, // N1 element 1 value
                0, // N2 element count
                0, // N4 element count
                0, // N8 element count
                2, // Data 2 count
                0, 0, 0x25, 0x2c // CRC16
            ]
        );

        let (bytes_decoded, decoded_packet) = Codec8Packet::decode(&buf[..bytes_encoded]).unwrap();
        assert_eq!(bytes_encoded, bytes_decoded);

        assert_eq!(packet, decoded_packet);
    }

    fn sample_frame_with_io() -> AvlDataRecord<Codec8IoElement> {
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
            io_element: Codec8IoElement {
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
            },
        }
    }

    fn sample_frame_without_io() -> AvlDataRecord<Codec8IoElement> {
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
            io_element: Codec8IoElement {
                event_io_id: 2,
                total_io_count: 0,
                n1_elements: StackVec::new(),
                n2_elements: StackVec::new(),
                n4_elements: StackVec::new(),
                n8_elements: StackVec::new(),
            },
        }
    }

    #[test]
    fn encodes_packet_header_and_crc_correctly() {
        let packet = Codec8Packet(StackVec::from_slice(&[sample_frame_without_io()]).unwrap());
        dbg!(&packet);

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
        let packet = Codec8Packet(
            StackVec::from_slice(&[sample_frame_with_io(), sample_frame_without_io()]).unwrap(),
        );

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
