use std::io::prelude::*;
use std::io::{Error, ErrorKind};

pub struct VariableSizeByteWriter {
    buf: Vec<u8>,
    bits: u32,
}

impl VariableSizeByteWriter {
    pub fn new(cap: usize) -> VariableSizeByteWriter {
        VariableSizeByteWriter {
            buf: vec![0; cap],
            bits: 0,
        }
    }

    #[inline]
    pub fn complete_bytes(&mut self) -> usize {
        (self.bits / 8) as usize
    }

    #[inline]
    pub fn partial_bits(&mut self) -> u32 {
        (self.bits % 8)
    }

    pub fn get_complete_bytes(&mut self) -> &[u8] {
        let bytes = self.complete_bytes();
        &self.buf[..bytes]
    }

    pub fn get_all_bytes(&mut self, partial_bits: &mut u32) -> &[u8] {
        let bytes = self.complete_bytes();
        *partial_bits = self.partial_bits();
        if *partial_bits > 0 {
            &self.buf[..bytes + 1]
        } else {
            &self.buf[..bytes]
        }
    }

    pub fn get_partial_byte(&mut self) -> Option<(u8, u32)> {
        let bytes = self.complete_bytes();
        let partial_bits = self.partial_bits();
        let partial = self.buf[bytes];
        if partial_bits > 0 {
            Some((partial, partial_bits as u32))
        } else {
            None
        }
    }

    pub fn erase_complete_bytes(&mut self) {
        let bytes = self.complete_bytes();
        let possible_partial = self.buf[bytes];
        self.buf[..bytes + 1].iter_mut().for_each(|n| *n = 0);
        self.buf[0] = possible_partial;
        self.bits = self.partial_bits() as u32;
    }

    pub fn erase_all_bytes(&mut self) {
        let bytes = self.complete_bytes();
        self.buf[..bytes + 1].iter_mut().for_each(|n| *n = 0);
        self.bits = 0;
    }

    pub fn move_range_to_start(&mut self, from: usize, to: usize) {
        let mut offset = 0;
        while from + offset < to {
            self.buf[offset] = self.buf[from + offset];
        }
        self.bits -= 8 * (to - from) as u32;
    }

    pub fn write_range<T>(&mut self, writer: &mut T, from: usize, to: usize, written: &mut usize) -> std::io::Result<()>
        where T: Write
    {
        *written = 0;
        while from + *written < to {
            match writer.write(&self.buf[from + *written..to]) {
                Ok(0) => return Err(Error::new(ErrorKind::WriteZero, "zero bytes written")),
                Ok(bytes) => *written += bytes,
                Err(ref err) if err.kind() == ErrorKind::Interrupted => {}
                Err(err) => return Err(err),
            }
        }
        Ok(())
    }

    pub fn insert_32(&mut self, variable: u32, bits: u32) {
        let byte: usize = self.complete_bytes();
        let offset: u32 = self.partial_bits();

		self.buf[byte] |= (variable << offset) as u8;
        let variable = variable >> 8 - offset;
		self.buf[byte + 1] |= variable as u8;
        let variable = variable >> 8;
		self.buf[byte + 2] |= variable as u8;
        let variable = variable >> 8;
		self.buf[byte + 3] |= variable as u8;
        let variable = variable >> 8;
		self.buf[byte + 4] |= variable as u8;

        self.bits += bits;
    }

    pub fn insert_32_unchecked(&mut self, variable: u32, bits: u32) {
        let byte: usize = self.complete_bytes();
        let offset: u32 = self.partial_bits();

        unsafe {
            let i = self.buf.get_unchecked_mut(byte as usize);
		    *i |= (variable << offset) as u8;
        }
        let variable = variable >> 8 - offset;
        unsafe {
            let i = self.buf.get_unchecked_mut(byte + 1 as usize);
		    *i |= variable as u8;
        }
        let variable = variable >> 8;
        unsafe {
            let i = self.buf.get_unchecked_mut(byte + 2 as usize);
		    *i |= variable as u8;
        }
        let variable = variable >> 8;
        unsafe {
            let i = self.buf.get_unchecked_mut(byte + 3 as usize);
		    *i |= variable as u8;
        }
        let variable = variable >> 8;
        unsafe {
            let i = self.buf.get_unchecked_mut(byte + 4 as usize);
		    *i |= variable as u8;
        }

        self.bits += bits;
    }

    pub fn insert_16(&mut self, variable: u16, bits: u32) {
        let byte: usize = self.complete_bytes();
        let offset: u32 = self.partial_bits();

		self.buf[byte] |= (variable << offset) as u8;
        let variable = variable >> 8 - offset;
		self.buf[byte + 1] |= variable as u8;
        let variable = variable >> 8;
		self.buf[byte + 2] |= variable as u8;

        self.bits += bits;
    }

    pub fn insert_16_unchecked(&mut self, variable: u16, bits: u32) {
        let byte: usize = self.complete_bytes();
        let offset: u32 = self.partial_bits();

        unsafe {
            let i = self.buf.get_unchecked_mut(byte as usize);
		    *i |= (variable << offset) as u8;
        }
        let variable = variable >> 8 - offset;
        unsafe {
            let i = self.buf.get_unchecked_mut(byte + 1 as usize);
		    *i |= variable as u8;
        }
        let variable = variable >> 8;
        unsafe {
            let i = self.buf.get_unchecked_mut(byte + 2 as usize);
		    *i |= variable as u8;
        }

        self.bits += bits;
    }
}





#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complete_bytes() {
        let mut writer = VariableSizeByteWriter::new(6);
        writer.buf[3] = 0xFF;
        writer.buf[4] = 0xF;
        writer.bits = 36;
        assert_eq!(writer.complete_bytes(), 4);
    }

    #[test]
    fn test_partial_bits() {
        let mut writer = VariableSizeByteWriter::new(6);
        writer.buf[3] = 0xFF;
        writer.buf[4] = 0xF;
        writer.bits = 36;
        assert_eq!(writer.partial_bits(), 4);
    }

    #[test]
    fn test_get_complete_bytes() {
        let mut writer = VariableSizeByteWriter::new(16);
        writer.buf[0] = 0xAA;
        writer.buf[3] = 0xFF;
        writer.buf[4] = 0xF;
        writer.bits = 36;
        assert_eq!(writer.get_complete_bytes(), [0xAA, 0, 0, 0xFF]);
    }

    #[test]
    fn test_get_all_bytes() {
        let mut writer = VariableSizeByteWriter::new(16);
        writer.buf[0] = 0xAA;
        writer.buf[3] = 0xFF;
        writer.buf[4] = 0xF;
        writer.bits = 36;
        let mut partial_bits = 0;
        assert_eq!(writer.get_all_bytes(&mut partial_bits), [0xAA, 0, 0, 0xFF, 0xF]);
        assert_eq!(partial_bits, 4);
    }

    #[test]
    fn test_get_partial_byte() {
        let mut writer = VariableSizeByteWriter::new(16);
        writer.buf[0] = 0xAA;
        writer.buf[3] = 0xFF;
        writer.buf[4] = 0xF;
        writer.bits = 36;
        let ret = writer.get_partial_byte();
        assert_eq!(ret, Some((0xF, 4)));

        let mut writer = VariableSizeByteWriter::new(16);
        writer.buf[0] = 0xAA;
        writer.buf[3] = 0xFF;
        writer.bits = 32;
        let ret = writer.get_partial_byte();
        assert_eq!(ret, None);
    }

    #[test]
    fn test_erase_complete_bytes() {
        let mut writer = VariableSizeByteWriter::new(6);
        writer.buf[3] = 0xFF;
        writer.buf[4] = 0xF;
        writer.bits = 36;
        writer.erase_complete_bytes();
        assert_eq!(writer.bits, 4);
        assert_eq!(writer.buf[..], [0xF, 0, 0, 0, 0, 0]);

        let mut writer = VariableSizeByteWriter::new(6);
        writer.buf[3] = 0xFF;
        writer.bits = 32;
        writer.erase_complete_bytes();
        assert_eq!(writer.bits, 0);
        assert_eq!(writer.buf[..], [0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_erase_all_bytes() {
        let mut writer = VariableSizeByteWriter::new(6);
        writer.buf[3] = 0xFF;
        writer.buf[4] = 0xF;
        writer.bits = 36;
        writer.erase_all_bytes();
        assert_eq!(writer.bits, 0);
        assert_eq!(writer.buf[..], [0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_move_range_to_start() {
        let mut writer = VariableSizeByteWriter::new(12);
        writer.buf[10] = 0xAB;
        writer.buf[11] = 0xF;
        writer.bits = 92;
        writer.move_range_to_start(7, 12);
        assert_eq!(writer.bits, 44);
        assert_eq!(writer.buf[..], [0, 0, 0, 0, 0xAB, 0xF]);
    }

    #[test]
    fn test_write_range() {
        let mut writer = VariableSizeByteWriter::new(6);
        writer.buf[3] = 0xFF;
        writer.buf[4] = 0xF;
        writer.bits = 36;

        let mut target = std::io::Cursor::new(vec![]);
        let mut written: usize = 0;
        writer.write_range(&mut target, 0, 4, &mut written).unwrap();
        assert_eq!(written, 4);
        assert_eq!(&target.get_ref()[..4], [0, 0, 0, 0xFF]);

        let mut target = std::io::Cursor::new(vec![]);
        let mut written: usize = 0;
        writer.write_range(&mut target, 2, 4, &mut written).unwrap();
        assert_eq!(written, 2);
        assert_eq!(&target.get_ref()[..2], [0, 0xFF]);
    }

    #[test]
    fn test_insert_32() {
        let mut writer = VariableSizeByteWriter::new(16);
        writer.insert_32(0xF, 4);
        assert_eq!(writer.buf[0..2], [0xF, 0]);
        assert_eq!(writer.bits, 4);

        writer.insert_32(0xFA, 8);
        assert_eq!(writer.buf[0..3], [0xAF, 0xF, 0]);
        assert_eq!(writer.bits, 12);

        writer.insert_32(0x1FFFBB, 21);
        assert_eq!(writer.buf[0..6], [0xAF, 0xBF, 0xFB, 0xFF, 0x1, 0]);
        assert_eq!(writer.bits, 33);
    }

    #[test]
    fn test_insert_32_unchecked() {
        let mut writer = VariableSizeByteWriter::new(16);
        writer.insert_32_unchecked(0xF, 4);
        assert_eq!(writer.buf[0..2], [0xF, 0]);
        assert_eq!(writer.bits, 4);

        writer.insert_32_unchecked(0xFA, 8);
        assert_eq!(writer.buf[0..3], [0xAF, 0xF, 0]);
        assert_eq!(writer.bits, 12);

        writer.insert_32_unchecked(0x1FFFBB, 21);
        assert_eq!(writer.buf[0..6], [0xAF, 0xBF, 0xFB, 0xFF, 0x1, 0]);
        assert_eq!(writer.bits, 33);
    }

    #[test]
    fn test_insert_16() {
        let mut writer = VariableSizeByteWriter::new(16);
        writer.insert_16(0xF, 4);
        assert_eq!(writer.buf[0..2], [0xF, 0]);
        assert_eq!(writer.bits, 4);

        writer.insert_16(0xFA, 8);
        assert_eq!(writer.buf[0..3], [0xAF, 0xF, 0]);
        assert_eq!(writer.bits, 12);

        writer.insert_16(0x1FBB, 13);
        assert_eq!(writer.buf[0..6], [0xAF, 0xBF, 0xFB, 0x1, 0, 0]);
        assert_eq!(writer.bits, 25);
    }

    #[test]
    fn test_insert_16_unchecked() {
        let mut writer = VariableSizeByteWriter::new(16);
        writer.insert_16_unchecked(0xF, 4);
        assert_eq!(writer.buf[0..2], [0xF, 0]);
        assert_eq!(writer.bits, 4);

        writer.insert_16_unchecked(0xFA, 8);
        assert_eq!(writer.buf[0..3], [0xAF, 0xF, 0]);
        assert_eq!(writer.bits, 12);

        writer.insert_16_unchecked(0x1FBB, 13);
        assert_eq!(writer.buf[0..6], [0xAF, 0xBF, 0xFB, 0x1, 0, 0]);
        assert_eq!(writer.bits, 25);
    }
}