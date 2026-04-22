use crate::{
    AvlCodec, AvlIoElement, StackVec,
    avl::{AvlDataRecord, AvlN1Element, AvlN2Element, AvlN4Element, AvlN8Element, AvlNxElement},
    crc16,
    error::AvlError,
};

pub const CODEC8_EXTENDED_TYPE_ID: u8 = 0x8e;

#[derive(Clone, Debug, PartialEq)]
pub struct Codec8ExtendedPacket(pub StackVec<AvlDataRecord<Codec8ExtendedIoElement>, 32>);

impl AvlCodec for Codec8ExtendedPacket {
    fn size(&self) -> usize {
        self.0.iter().map(|f| f.size()).sum::<usize>() + 15 // preamble + data_field_length + codec_id + data_1_count + data_2_count + avl_data_records + CRC16
    }

    fn encode(&self, buf: &mut [u8]) -> Result<usize, AvlError> {
        buf[0..4].copy_from_slice(&[0, 0, 0, 0]); // Preamble

        let data_field_length = self.size() - 12; // without preamble and data_field_length and CRC16

        buf[4..8].copy_from_slice(&(data_field_length as u32).to_be_bytes());

        buf[8] = CODEC8_EXTENDED_TYPE_ID;

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
pub struct Codec8ExtendedIoElement {
    pub event_io_id: u16,
    pub total_io_count: u16,
    pub n1_elements: StackVec<AvlN1Element<u16>, 16>,
    pub n2_elements: StackVec<AvlN2Element<u16>, 16>,
    pub n4_elements: StackVec<AvlN4Element<u16>, 16>,
    pub n8_elements: StackVec<AvlN8Element<u16>, 16>,
    pub nx_elements: StackVec<AvlNxElement<u16>, 16>,
}

impl AvlIoElement for Codec8ExtendedIoElement {
    fn size(&self) -> usize {
        6 + (self.n1_elements.len() * AvlN1Element::<u16>::size())
            + (self.n2_elements.len() * AvlN2Element::<u16>::size())
            + (self.n4_elements.len() * AvlN4Element::<u16>::size())
            + (self.n8_elements.len() * AvlN8Element::<u16>::size())
            + (self.nx_elements.iter().map(|e| e.size()).sum::<usize>())
    }

    fn encode(&self, buf: &mut [u8]) -> Result<usize, AvlError> {
        let mut offset = 0;

        buf[offset..offset + 2].copy_from_slice(&self.event_io_id.to_be_bytes());
        offset += 2;

        // Total IO count
        buf[offset..offset + 2].copy_from_slice(&self.total_io_count.to_be_bytes());
        offset += 2;

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

        let event_io_id = u16::from_be_bytes(buf[offset..offset + 2].try_into().unwrap());
        offset += 2;

        let total_io_count = u16::from_be_bytes(buf[offset..offset + 2].try_into().unwrap());
        offset += 2;

        let n1_io_count = u16::from_be_bytes(buf[offset..offset + 2].try_into().unwrap());
        offset += 2;

        let mut n1_elements = StackVec::new();

        for _ in 0..n1_io_count {
            let (bytes_read, n1_element) = AvlN1Element::decode(&buf[offset..])?;
            n1_elements.push(n1_element).unwrap();
            offset += bytes_read;
        }

        let n2_io_count = u16::from_be_bytes(buf[offset..offset + 2].try_into().unwrap());
        offset += 2;

        let mut n2_elements = StackVec::new();

        for _ in 0..n2_io_count {
            let (bytes_read, n2_element) = AvlN2Element::decode(&buf[offset..])?;
            n2_elements.push(n2_element).unwrap();
            offset += bytes_read;
        }

        let n4_io_count = u16::from_be_bytes(buf[offset..offset + 2].try_into().unwrap());
        offset += 2;

        let mut n4_elements = StackVec::new();

        for _ in 0..n4_io_count {
            let (bytes_read, n4_element) = AvlN4Element::decode(&buf[offset..])?;
            n4_elements.push(n4_element).unwrap();
            offset += bytes_read;
        }

        let n8_io_count = u16::from_be_bytes(buf[offset..offset + 2].try_into().unwrap());
        offset += 2;

        let mut n8_elements = StackVec::new();

        for _ in 0..n8_io_count {
            let (bytes_read, n8_element) = AvlN8Element::decode(&buf[offset..])?;
            n8_elements.push(n8_element).unwrap();
            offset += bytes_read;
        }

        let nx_io_count = u16::from_be_bytes(buf[offset..offset + 2].try_into().unwrap());
        offset += 2;

        let mut nx_elements = StackVec::new();

        for _ in 0..nx_io_count {
            let (bytes_read, nx_element) = AvlNxElement::decode(&buf[offset..])?;
            nx_elements.push(nx_element).unwrap();
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
                nx_elements,
            },
        ))
    }
}
