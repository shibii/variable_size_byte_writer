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
    pub fn all_bytes(&mut self) -> usize {
        ((self.bits + 7) / 8) as usize
    }

    #[inline]
    pub fn partial_bits(&mut self) -> u32 {
        (self.bits % 8)
    }

    #[inline]
    pub fn padding(&mut self) -> u32 {
        let partial_bits = self.bits % 8;
        if partial_bits > 0 {
            8 - partial_bits
        } else {
            0
        }
    }

    #[inline]
    pub fn get_complete_bytes(&mut self) -> &[u8] {
        let bytes = self.complete_bytes();
        &self.buf[..bytes]
    }

    #[inline]
    pub fn get_all_bytes(&mut self, partial_bits: &mut u32) -> &[u8] {
        let bytes = self.complete_bytes();
        *partial_bits = self.partial_bits();
        if *partial_bits > 0 {
            &self.buf[..bytes + 1]
        } else {
            &self.buf[..bytes]
        }
    }

    #[inline]
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

    #[inline]
    pub fn erase_complete_bytes(&mut self) {
        let bytes = self.complete_bytes();
        let possible_partial = self.buf[bytes];
        self.buf[..bytes + 1].iter_mut().for_each(|n| *n = 0);
        self.buf[0] = possible_partial;
        self.bits = self.partial_bits() as u32;
    }

    #[inline]
    pub fn erase_all_bytes(&mut self) {
        let bytes = self.complete_bytes();
        self.buf[..bytes + 1].iter_mut().for_each(|n| *n = 0);
        self.bits = 0;
    }

    #[inline]
    pub fn move_range_to_start(&mut self, from: usize, to: usize) {
        let mut offset = 0;
        while from + offset < to {
            self.buf[offset] = self.buf[from + offset];
            self.buf[from + offset] = 0;
            offset += 1;
        }
        self.bits -= 8 * (to - from) as u32;
    }

	pub fn write_32<T>(&mut self, writer: &mut T, variable: u32, bits: u32) -> std::io::Result<()>
        where T: Write
    {
        if !self.can_insert_32() {
            self.try_flush_complete_bytes(writer)?;
        }
        self.insert_32_unchecked(variable, bits);
        Ok(())
    }

	pub fn write_16<T>(&mut self, writer: &mut T, variable: u16, bits: u32) -> std::io::Result<()>
        where T: Write
    {
        if !self.can_insert_16() {
            self.try_flush_complete_bytes(writer)?;
        }
        self.insert_16_unchecked(variable, bits);
        Ok(())
    }

    pub fn try_flush_complete_bytes<T>(&mut self, writer: &mut T) -> std::io::Result<()>
        where T: Write
    {
        let complete = self.complete_bytes();
        let mut written = 0;
        let result = self.write_range(writer, 0, complete, &mut written);
        match result {
            Ok(()) => self.erase_complete_bytes(),
            Err(err) => {
                if written > 0 {
                    self.move_range_to_start(written, complete + 1);
                } else {
                    return Err(err)
                }
            }
        }
        Ok(())
    }

    pub fn try_flush_all_bytes<T>(&mut self, writer: &mut T, padding: &mut u32) -> std::io::Result<()>
        where T: Write
    {
        let bytes = self.all_bytes();
        *padding = self.padding();
        let mut written = 0;
        let result = self.write_range(writer, 0, bytes, &mut written);
        match result {
            Ok(()) => self.erase_all_bytes(),
            Err(err) => {
                if written > 0 {
                    self.move_range_to_start(written, bytes + 1);
                } else {
                    return Err(err)
                }
            }
        }
        Ok(())
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

    #[inline]
    pub fn can_insert_32(&mut self) -> bool {
        if self.complete_bytes() + 4 >= self.buf.len() {
            false
        } else {
            true
        }
    }

    #[inline]
    pub fn can_insert_16(&mut self) -> bool {
        if self.complete_bytes() + 2 >= self.buf.len() {
            false
        } else {
            true
        }
    }

    #[inline]
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

    #[inline]
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

    #[inline]
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

    #[inline]
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
    fn test_all_bytes() {
        let mut writer = VariableSizeByteWriter::new(6);
        writer.bits = 24;
        assert_eq!(writer.all_bytes(), 3);
        writer.bits = 25;
        assert_eq!(writer.all_bytes(), 4);
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
    fn test_padding() {
        let mut writer = VariableSizeByteWriter::new(6);
        writer.bits = 33;
        assert_eq!(writer.padding(), 7);

        writer.bits = 32;
        assert_eq!(writer.padding(), 0);
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
        let mut writer = VariableSizeByteWriter::new(6);
        writer.buf[4] = 0xAB;
        writer.buf[5] = 0xF;
        writer.bits = 44;
        writer.move_range_to_start(3, 6);
        assert_eq!(writer.bits, 20);
        assert_eq!(writer.buf[..], [0, 0xAB, 0xF, 0, 0, 0]);
    }

    #[test]
    fn test_write_32() {
        let mut writer = VariableSizeByteWriter::new(6);
        let mut target = std::io::Cursor::new(vec![]);

        writer.write_32(&mut target, 0x1F0, 9).unwrap();
        assert_eq!(writer.buf[..], [0xF0, 0x1, 0, 0, 0, 0]);
        assert_eq!(writer.bits, 9);
        assert_eq!(&target.get_ref()[..], []);

        writer.write_32(&mut target, 0x78, 9).unwrap();
        assert_eq!(writer.buf[..], [0xF0, 0xF1, 0, 0, 0, 0]);
        assert_eq!(writer.bits, 18);
        assert_eq!(&target.get_ref()[..], []);

        writer.write_32(&mut target, 0x1F7, 9).unwrap();
        assert_eq!(writer.buf[..], [0xDC, 0x7, 0, 0, 0, 0]);
        assert_eq!(writer.bits, 11);
        assert_eq!(&target.get_ref()[..], [0xF0, 0xF1]);
    }

    #[test]
    fn test_write_16() {
        let mut writer = VariableSizeByteWriter::new(4);
        let mut target = std::io::Cursor::new(vec![]);

        writer.write_16(&mut target, 0x1F0, 9).unwrap();
        assert_eq!(writer.buf[..], [0xF0, 0x1, 0, 0]);
        assert_eq!(writer.bits, 9);
        assert_eq!(&target.get_ref()[..], []);

        writer.write_16(&mut target, 0x78, 9).unwrap();
        assert_eq!(writer.buf[..], [0xF0, 0xF1, 0, 0]);
        assert_eq!(writer.bits, 18);
        assert_eq!(&target.get_ref()[..], []);

        writer.write_16(&mut target, 0x1F7, 9).unwrap();
        assert_eq!(writer.buf[..], [0xDC, 0x7, 0, 0]);
        assert_eq!(writer.bits, 11);
        assert_eq!(&target.get_ref()[..], [0xF0, 0xF1]);
    }

    #[test]
    fn test_try_flush_complete_bytes() {
        let mut writer = VariableSizeByteWriter::new(6);
        writer.buf[0] = 0xFF;
        writer.buf[1] = 0xA;
        writer.buf[2] = 0xAB;
        writer.buf[3] = 0xC;
        writer.bits = 28;
        let mut target = std::io::Cursor::new(vec![]);
        writer.try_flush_complete_bytes(&mut target).unwrap();
        assert_eq!(&target.get_ref()[..3], [0xFF, 0xA, 0xAB]);
        assert_eq!(writer.bits, 4);
    }

    #[test]
    fn test_try_flush_all_bytes() {
        let mut writer = VariableSizeByteWriter::new(6);
        writer.buf[0] = 0xFF;
        writer.buf[1] = 0xA;
        writer.buf[2] = 0xAB;
        writer.buf[3] = 0xC;
        writer.bits = 28;
        let mut target = std::io::Cursor::new(vec![]);
        let mut padding = 0;
        writer.try_flush_all_bytes(&mut target, &mut padding).unwrap();
        assert_eq!(&target.get_ref()[..4], [0xFF, 0xA, 0xAB, 0xC]);
        assert_eq!(writer.bits, 0);
        assert_eq!(padding, 4);
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
    fn test_can_insert_32() {
        let mut writer = VariableSizeByteWriter::new(6);
        writer.bits = 15;
        assert_eq!(writer.can_insert_32(), true);
        writer.bits = 17;
        assert_eq!(writer.can_insert_32(), false);
    }

    #[test]
    fn test_can_insert_16() {
        let mut writer = VariableSizeByteWriter::new(6);
        writer.bits = 31;
        assert_eq!(writer.can_insert_16(), true);
        writer.bits = 33;
        assert_eq!(writer.can_insert_16(), false);
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