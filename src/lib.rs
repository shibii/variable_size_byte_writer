
use std::io::prelude::*;
use std::io::{Error, ErrorKind};

/// `VariableSizeByteWriter` provides functions for writing variable-size bytes
/// into `io::Write` traited targets.
///
/// Writes are internally buffered and so the usage of any additional buffering
/// such as `std::io::BufWriter` is not recommended.
///
/// Note that `VariableSizeByteWriter` does not flush its internal buffer when
/// dropped.
///
/// # Examples
///
/// Writing some unconventionally sized bytes into `Vec<u8>`
///
/// ```
/// use variable_size_byte_writer::*;
///
/// let mut target = Vec::new();
/// let mut writer = VariableSizeByteWriter::new();
/// let bytes = [(0x3F, 6),(0x1AFF, 13),(0x7, 3)];
///
/// bytes
///     .iter()
///     .for_each(|&(byte, bits)|
///         writer.write_16(&mut target, byte, bits).unwrap()
///     );
///
/// let mut padding = 0;
/// writer
///     .flush_all_bytes(&mut target, &mut padding)
///     .unwrap();
///
/// assert_eq!(padding, 2);
/// assert_eq!(target[..], [0xFF, 0xBF, 0x3E]);
/// ```
///
/// Writing a series of 7bit bytes into a file
///
/// ```
/// use std::fs::File;
/// use variable_size_byte_writer::*;
///
/// # fn f() -> std::io::Result<()> {
/// let mut writer = VariableSizeByteWriter::new();
/// let mut file = File::create("path").unwrap();
///
/// for variable in 0..0x8F {
///     writer.write_8(&mut file, variable, 7).unwrap();
/// }
///
/// let mut padding = 0;
/// writer
///     .flush_all_bytes(&mut file, &mut padding)
///     .unwrap();
/// # Ok(())
/// # }
/// ```
pub struct VariableSizeByteWriter {
    buf: Vec<u8>,
    bits: u32,
}

impl VariableSizeByteWriter {
    /// Creates a new instance of `VariableSizeByteWriter` with a default
    /// internal buffer size.
    ///
    /// # Examples
    ///
    /// ```
    /// use variable_size_byte_writer::*;
    ///
    /// let writer = VariableSizeByteWriter::new();
    /// ```
    pub fn new() -> Self {
        VariableSizeByteWriter::with_specified_capacity(8192)
    }

    /// Creates a new instance of `VariableSizeByteWriter` with a specific
    /// internal buffer size.
    ///
    /// # Examples
    ///
    /// ```
    /// use variable_size_byte_writer::*;
    ///
    /// let writer = VariableSizeByteWriter::with_specified_capacity(4096);
    /// ```
    pub fn with_specified_capacity(cap: usize) -> Self {
        VariableSizeByteWriter {
            buf: vec![0; cap],
            bits: 0,
        }
    }

    #[inline]
    fn complete_bytes(&self) -> usize {
        (self.bits / 8) as usize
    }

    #[inline]
    fn all_bytes(&self) -> usize {
        ((self.bits + 7) / 8) as usize
    }

    #[inline]
    fn partial_bits(&self) -> u32 {
        (self.bits % 8)
    }

    #[inline]
    fn padding(&self) -> u32 {
        (8 - self.partial_bits()) % 8
    }

    #[inline]
    fn erase_complete_bytes(&mut self) {
        let bytes = self.complete_bytes();
        let possible_partial = self.buf[bytes];
        self.buf[..bytes + 1].iter_mut().for_each(|n| *n = 0);
        self.buf[0] = possible_partial;
        self.bits = self.partial_bits() as u32;
    }

    #[inline]
    fn erase_all_bytes(&mut self) {
        let bytes = self.complete_bytes();
        self.buf[..bytes + 1].iter_mut().for_each(|n| *n = 0);
        self.bits = 0;
    }

    #[inline]
    fn move_range_to_start(&mut self, from: usize, to: usize) {
        let mut offset = 0;
        while from + offset < to {
            self.buf[offset] = self.buf[from + offset];
            self.buf[from + offset] = 0;
            offset += 1;
        }
        self.bits -= 8 * (to - from) as u32;
    }

    /// Writes a variable-sized byte `variable` with a specific length of `bits`
    /// into the given `target`.
    ///
    /// As with all the `write` functions, the operation is buffered and the
    /// buffer must eventually be flushed with the `flush_all_bytes` function.
    ///
    /// The given byte can be no longer than 64 bits.
    /// The padding of the variable must be clean as in all zeroes.
    ///
    /// The function might fail once the internal buffer fills up and is flushed
    /// into the given target.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::fs::File;
    /// use variable_size_byte_writer::*;
    ///
    /// # fn f() -> std::io::Result<()> {
    /// let mut writer = VariableSizeByteWriter::with_specified_capacity(4096);
    /// let mut file = File::create("path")?;
    ///
    /// writer.write_64(&mut file, 0x71CFFABFF, 35)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn write_64<T>(&mut self, writer: &mut T, variable: u64, bits: u32) -> std::io::Result<()>
    where
        T: Write,
    {
        if !self.can_insert_64() {
            self.flush_complete_bytes(writer)?;
        }
        self.insert_64(variable, bits);
        Ok(())
    }

    /// Faster than `write_64` but the given byte can be no longer than 32 bits.
    pub fn write_32<T>(&mut self, writer: &mut T, variable: u32, bits: u32) -> std::io::Result<()>
    where
        T: Write,
    {
        if !self.can_insert_32() {
            self.flush_complete_bytes(writer)?;
        }
        self.insert_32(variable, bits);
        Ok(())
    }

    /// Faster than `write_32` but the given byte can be no longer than 16 bits.
    pub fn write_16<T>(&mut self, writer: &mut T, variable: u16, bits: u32) -> std::io::Result<()>
    where
        T: Write,
    {
        if !self.can_insert_16() {
            self.flush_complete_bytes(writer)?;
        }
        self.insert_16(variable as u16, bits);
        Ok(())
    }

    /// Faster than `write_16` but the given byte can be no longer than 8 bits.
    pub fn write_8<T>(&mut self, writer: &mut T, variable: u8, bits: u32) -> std::io::Result<()>
    where
        T: Write,
    {
        if !self.can_insert_8() {
            self.flush_complete_bytes(writer)?;
        }
        self.insert_8(variable as u16, bits);
        Ok(())
    }

    fn flush_complete_bytes<T>(&mut self, writer: &mut T) -> std::io::Result<()>
    where
        T: Write,
    {
        let complete = self.complete_bytes();
        let mut written = 0;
        let result = self.write_range(writer, 0, complete, &mut written);
        match result {
            Ok(()) => self.erase_complete_bytes(),
            Err(err) => if written > 0 {
                self.move_range_to_start(written, complete + 1);
            } else {
                return Err(err);
            },
        }
        Ok(())
    }

    /// Flushes the remaining internal buffer to the given `target`.
    ///
    /// The function might fail, successfully flushing none or some of the
    /// internal buffer.
    /// If the flush fails, the internal buffer remains intact and contains
    /// the content that failed to flush.
    ///
    /// The padding required to fill the last partial byte can be captured
    /// into the argument `padding`.
    /// The padding is only valid if the function return without an error.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::fs::File;
    /// use variable_size_byte_writer::*;
    ///
    /// # fn f() -> std::io::Result<()> {
    /// let mut writer = VariableSizeByteWriter::with_specified_capacity(4096);
    /// let mut file = File::create("path")?;
    ///
    /// writer.write_64(&mut file, 0x71CFFABFF, 35)?;
    /// writer.write_64(&mut file, 0xFFA, 16)?;
    /// writer.write_64(&mut file, 0xF1CFFABCD, 39)?;
    ///
    /// let mut padding = 0;
    /// writer.flush_all_bytes(&mut file, &mut padding)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn flush_all_bytes<T>(
        &mut self,
        writer: &mut T,
        padding: &mut u32,
    ) -> std::io::Result<()>
    where
        T: Write,
    {
        let bytes = self.all_bytes();
        *padding = self.padding();
        let mut written = 0;
        let result = self.write_range(writer, 0, bytes, &mut written);
        match result {
            Ok(()) => self.erase_all_bytes(),
            Err(err) => if written > 0 {
                self.move_range_to_start(written, bytes + 1);
            } else {
                return Err(err);
            },
        }
        Ok(())
    }

    fn write_range<T>(
        &self,
        writer: &mut T,
        from: usize,
        to: usize,
        written: &mut usize,
    ) -> std::io::Result<()>
    where
        T: Write,
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
    fn can_insert_64(&mut self) -> bool {
        self.complete_bytes() + 8 < self.buf.len()
    }

    #[inline]
    fn can_insert_32(&mut self) -> bool {
        self.complete_bytes() + 4 < self.buf.len()
    }

    #[inline]
    fn can_insert_16(&mut self) -> bool {
        self.complete_bytes() + 2 < self.buf.len()
    }

    #[inline]
    fn can_insert_8(&mut self) -> bool {
        self.complete_bytes() + 1 < self.buf.len()
    }

    #[inline]
    fn insert_64(&mut self, variable: u64, bits: u32) {
        let byte: usize = self.complete_bytes();
        let offset: u32 = self.partial_bits();

        unsafe {
            *self.buf.get_unchecked_mut(byte as usize) |= (variable << offset) as u8;
            let mut variable = variable >> (8 - offset);

            (1..9).for_each(|i| {
                *self.buf.get_unchecked_mut(byte + i as usize) = variable as u8;
                variable >>= 8;
            })
        }

        self.bits += bits;
    }

    #[inline]
    fn insert_32(&mut self, variable: u32, bits: u32) {
        let byte: usize = self.complete_bytes();
        let offset: u32 = self.partial_bits();

        unsafe {
            *self.buf.get_unchecked_mut(byte as usize) |= (variable << offset) as u8;
            let variable = variable >> (8 - offset);
            *self.buf.get_unchecked_mut(byte + 1 as usize) = variable as u8;
            let variable = variable >> 8;
            *self.buf.get_unchecked_mut(byte + 2 as usize) = variable as u8;
            let variable = variable >> 8;
            *self.buf.get_unchecked_mut(byte + 3 as usize) = variable as u8;
            let variable = variable >> 8;
            *self.buf.get_unchecked_mut(byte + 4 as usize) = variable as u8;
        }

        self.bits += bits;
    }

    #[inline]
    fn insert_16(&mut self, variable: u16, bits: u32) {
        let byte: usize = self.complete_bytes();
        let offset: u32 = self.partial_bits();

        unsafe {
            *self.buf.get_unchecked_mut(byte as usize) |= (variable << offset) as u8;
            let variable = variable >> (8 - offset);
            *self.buf.get_unchecked_mut(byte + 1 as usize) = variable as u8;
            let variable = variable >> 8;
            *self.buf.get_unchecked_mut(byte + 2 as usize) = variable as u8;
        }

        self.bits += bits;
    }

    #[inline]
    fn insert_8(&mut self, variable: u16, bits: u32) {
        let byte: usize = self.complete_bytes();
        let offset: u32 = self.partial_bits();

        unsafe {
            *self.buf.get_unchecked_mut(byte as usize) |= (variable << offset) as u8;
            let variable = variable >> (8 - offset);
            *self.buf.get_unchecked_mut(byte + 1 as usize) = variable as u8;
        }

        self.bits += bits;
    }
}





#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let writer = VariableSizeByteWriter::new();
        assert_eq!(writer.bits, 0);
        assert_eq!(writer.buf.len(), 8192);
    }

    #[test]
    fn test_with_specified_capacity() {
        let writer = VariableSizeByteWriter::with_specified_capacity(1024);
        assert_eq!(writer.bits, 0);
        assert_eq!(writer.buf.len(), 1024);
    }

    #[test]
    fn test_default() {
        let mut writer = VariableSizeByteWriter::new();
        writer.bits = 31;
        assert_eq!(writer.complete_bytes(), 3);
        writer.bits = 32;
        assert_eq!(writer.complete_bytes(), 4);
    }

    #[test]
    fn test_complete_bytes() {
        let mut writer = VariableSizeByteWriter::new();
        writer.bits = 31;
        assert_eq!(writer.complete_bytes(), 3);
        writer.bits = 32;
        assert_eq!(writer.complete_bytes(), 4);
    }

    #[test]
    fn test_all_bytes() {
        let mut writer = VariableSizeByteWriter::new();
        writer.bits = 31;
        assert_eq!(writer.all_bytes(), 4);
        writer.bits = 32;
        assert_eq!(writer.all_bytes(), 4);
    }

    #[test]
    fn test_partial_bits() {
        let mut writer = VariableSizeByteWriter::new();
        writer.bits = 31;
        assert_eq!(writer.partial_bits(), 7);
        writer.bits = 32;
        assert_eq!(writer.partial_bits(), 0);
    }

    #[test]
    fn test_padding() {
        let mut writer = VariableSizeByteWriter::new();
        writer.bits = 33;
        assert_eq!(writer.padding(), 7);
        writer.bits = 32;
        assert_eq!(writer.padding(), 0);
    }

    #[test]
    fn test_erase_complete_bytes() {
        let mut writer = VariableSizeByteWriter::with_specified_capacity(6);
        writer.buf[3] = 0xFF;
        writer.buf[4] = 0xF;
        writer.bits = 36;
        writer.erase_complete_bytes();
        assert_eq!(writer.bits, 4);
        assert_eq!(writer.buf[..], [0xF, 0, 0, 0, 0, 0]);

        let mut writer = VariableSizeByteWriter::with_specified_capacity(6);
        writer.buf[3] = 0xFF;
        writer.bits = 32;
        writer.erase_complete_bytes();
        assert_eq!(writer.bits, 0);
        assert_eq!(writer.buf[..], [0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_erase_all_bytes() {
        let mut writer = VariableSizeByteWriter::with_specified_capacity(6);
        writer.buf[3] = 0xFF;
        writer.buf[4] = 0xF;
        writer.bits = 36;
        writer.erase_all_bytes();
        assert_eq!(writer.bits, 0);
        assert_eq!(writer.buf[..], [0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_move_range_to_start() {
        let mut writer = VariableSizeByteWriter::with_specified_capacity(6);
        writer.buf[4] = 0xAB;
        writer.buf[5] = 0xF;
        writer.bits = 44;
        writer.move_range_to_start(3, 6);
        assert_eq!(writer.bits, 20);
        assert_eq!(writer.buf[..], [0, 0xAB, 0xF, 0, 0, 0]);
    }

    #[test]
    fn test_write_64() {
        let mut writer = VariableSizeByteWriter::with_specified_capacity(10);
        let mut target = std::io::Cursor::new(vec![]);

        writer.write_64(&mut target, 0x1ABABFFFF, 33).unwrap();
        assert_eq!(writer.buf[..6], [0xFF, 0xFF, 0xAB, 0xAB, 1, 0]);
        assert_eq!(writer.bits, 33);
        assert_eq!(&target.get_ref()[..], []);

        writer.write_64(&mut target, 0x100000781, 33).unwrap();
        assert_eq!(writer.buf[..6], [0x3, 0x0F, 0, 0, 2, 0]);
        assert_eq!(writer.bits, 34);
        assert_eq!(&target.get_ref()[..], [0xFF, 0xFF, 0xAB, 0xAB]);
    }

    #[test]
    fn test_write_32() {
        let mut writer = VariableSizeByteWriter::with_specified_capacity(6);
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
        let mut writer = VariableSizeByteWriter::with_specified_capacity(4);
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
    fn test_write_8() {
        let mut writer = VariableSizeByteWriter::with_specified_capacity(2);
        let mut target = std::io::Cursor::new(vec![]);

        writer.write_8(&mut target, 0x5F, 7).unwrap();
        assert_eq!(writer.buf[..], [0x5F, 0]);
        assert_eq!(writer.bits, 7);
        assert_eq!(&target.get_ref()[..], []);

        writer.write_8(&mut target, 0x79, 7).unwrap();
        assert_eq!(writer.buf[..], [0xDF, 0x3C]);
        assert_eq!(writer.bits, 14);
        assert_eq!(&target.get_ref()[..], []);

        writer.write_8(&mut target, 0x7F, 7).unwrap();
        assert_eq!(writer.buf[..], [0xFC, 0x1F]);
        assert_eq!(writer.bits, 13);
        assert_eq!(&target.get_ref()[..], [0xDF]);
    }

    #[test]
    fn test_flush_complete_bytes() {
        let mut writer = VariableSizeByteWriter::new();
        writer.buf[0] = 0xFF;
        writer.buf[1] = 0xA;
        writer.buf[2] = 0xAB;
        writer.buf[3] = 0xC;
        writer.bits = 28;
        let mut target = std::io::Cursor::new(vec![]);
        writer.flush_complete_bytes(&mut target).unwrap();
        assert_eq!(&target.get_ref()[..3], [0xFF, 0xA, 0xAB]);
        assert_eq!(writer.bits, 4);
    }

    #[test]
    fn test_flush_all_bytes() {
        let mut writer = VariableSizeByteWriter::new();
        writer.buf[0] = 0xFF;
        writer.buf[1] = 0xA;
        writer.buf[2] = 0xAB;
        writer.buf[3] = 0xC;
        writer.bits = 28;
        let mut target = std::io::Cursor::new(vec![]);
        let mut padding = 0;
        writer
            .flush_all_bytes(&mut target, &mut padding)
            .unwrap();
        assert_eq!(&target.get_ref()[..4], [0xFF, 0xA, 0xAB, 0xC]);
        assert_eq!(writer.bits, 0);
        assert_eq!(padding, 4);
    }

    #[test]
    fn test_write_range() {
        let mut writer = VariableSizeByteWriter::new();
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
    fn test_can_insert_64() {
        let mut writer = VariableSizeByteWriter::with_specified_capacity(10);
        writer.bits = 15;
        assert_eq!(writer.can_insert_64(), true);
        writer.bits = 17;
        assert_eq!(writer.can_insert_64(), false);
    }

    #[test]
    fn test_can_insert_32() {
        let mut writer = VariableSizeByteWriter::with_specified_capacity(6);
        writer.bits = 15;
        assert_eq!(writer.can_insert_32(), true);
        writer.bits = 17;
        assert_eq!(writer.can_insert_32(), false);
    }

    #[test]
    fn test_can_insert_16() {
        let mut writer = VariableSizeByteWriter::with_specified_capacity(6);
        writer.bits = 31;
        assert_eq!(writer.can_insert_16(), true);
        writer.bits = 33;
        assert_eq!(writer.can_insert_16(), false);
    }

    #[test]
    fn test_can_insert_8() {
        let mut writer = VariableSizeByteWriter::with_specified_capacity(6);
        writer.bits = 39;
        assert_eq!(writer.can_insert_8(), true);
        writer.bits = 41;
        assert_eq!(writer.can_insert_8(), false);
    }

    #[test]
    fn test_insert_64() {
        let mut writer = VariableSizeByteWriter::new();
        writer.insert_64(0x1ABABFFFF, 33);
        assert_eq!(writer.buf[0..5], [0xFF, 0xFF, 0xAB, 0xAB, 1]);
        assert_eq!(writer.bits, 33);

        writer.insert_64(0x100000781, 33);
        assert_eq!(writer.buf[0..10], [0xFF, 0xFF, 0xAB, 0xAB, 0x3, 0x0F, 0, 0, 2, 0]);
        assert_eq!(writer.bits, 66);
    }

    #[test]
    fn test_insert_32() {
        let mut writer = VariableSizeByteWriter::new();
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
    fn test_insert_16() {
        let mut writer = VariableSizeByteWriter::new();
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
    fn test_insert_8() {
        let mut writer = VariableSizeByteWriter::new();
        writer.insert_8(0xF, 4);
        assert_eq!(writer.buf[0..2], [0xF, 0]);
        assert_eq!(writer.bits, 4);

        writer.insert_8(0xFA, 8);
        assert_eq!(writer.buf[0..3], [0xAF, 0xF, 0]);
        assert_eq!(writer.bits, 12);

        writer.insert_8(0x7B, 7);
        assert_eq!(writer.buf[0..4], [0xAF, 0xBF, 0x7, 0]);
        assert_eq!(writer.bits, 19);
    }
}
