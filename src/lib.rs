extern crate typenum;
extern crate typenum_loops;

use std::io::prelude::*;
use std::io::{Error, ErrorKind};

pub trait ByteSize: typenum_loops::Loop + typenum::Unsigned + typenum::NonZero {}

impl<T> ByteSize for T
where
    T: typenum_loops::Loop + typenum::Unsigned + typenum::NonZero {}

pub type Max8 = typenum::U1;
pub type Max16 = typenum::U2;
pub type Max24 = typenum::U3;
pub type Max32 = typenum::U4;
pub type Max40 = typenum::U5;
pub type Max48 = typenum::U6;
pub type Max56 = typenum::U7;
pub type Max64 = typenum::U8;

/// `VariableSizeByteWriter` provides functionality for writing variable-size bytes
/// into `io::Write` traited targets.
///
/// Writes are internally buffered and so the usage of any additional buffering
/// such as `std::io::BufWriter` is not recommended. The internal buffer is
/// flushed when the object is dropped but any errors that occur during the flushing
/// go unhandled. Manual flushing is therefore recommended.
///
/// # Examples
///
/// Writing some unconventionally sized bytes into `Vec<u8>`:
///
/// ```
/// use variable_size_byte_writer::*;
///
/// let mut target = Vec::new();
/// let mut writer = VariableSizeByteWriter::new(target);
/// let bytes = [(0x3F, 6),(0x1AFF, 13),(0x7, 3)];
///
/// bytes
///     .iter()
///     .for_each(|&(byte, bits)|
///         writer.write::<Max16>(byte, bits).unwrap()
///     );
/// ```
///
/// Writing a series of 7-bit bytes into a file, manually
/// flushing the internal buffer and capturing the
/// required bits to pad the last byte:
///
/// ```
/// use std::fs::File;
/// use variable_size_byte_writer::*;
///
/// # fn f() -> std::io::Result<()> {
/// let mut file = File::create("path").unwrap();
/// let mut writer = VariableSizeByteWriter::new(file);
///
/// for variable in 0..0x8F {
///     writer.write::<Max8>(variable, 7).unwrap();
/// }
///
/// let mut padding = 0;
/// writer.flush(&mut padding).unwrap();
/// # Ok(())
/// # }
/// ```
pub struct VariableSizeByteWriter<W>
where
    W: Write,
{
    buf: Vec<u8>,
    bits: u32,
    target: W
}

impl<W> Drop for VariableSizeByteWriter<W>
where
    W: Write,
{
    fn drop(&mut self) {
        let mut padding = 0;
        let _res = self.flush(&mut padding);
    }
}

impl<W> VariableSizeByteWriter<W>
where
    W: Write,
{
    /// Creates a new instance of `VariableSizeByteWriter`.
    ///
    /// The function takes a `io::Write` traited object `target`
    /// as an argument.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::fs::File;
    /// use variable_size_byte_writer::*;
    ///
    /// # fn f() -> std::io::Result<()> {
    /// let mut file = File::create("path").unwrap();
    /// let mut writer = VariableSizeByteWriter::new(file);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(target: W) -> Self {
        VariableSizeByteWriter::with_capacity(target, 8192)
    }

    /// Creates a new instance of `VariableSizeByteWriter`
    /// with non default internal buffer size.
    ///
    /// The function takes buffer capacity `cap` and `io::Write` traited
    /// object `target` as arguments.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::fs::File;
    /// use variable_size_byte_writer::*;
    ///
    /// # fn f() -> std::io::Result<()> {
    /// let mut file = File::create("path").unwrap();
    /// let mut writer = VariableSizeByteWriter::with_capacity(file, 4096);
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_capacity(target: W, cap: usize) -> Self {
        VariableSizeByteWriter {
            buf: vec![0; cap],
            bits: 0,
            target: target,
        }
    }

    /// Writes a variable-sized byte `variable` with a specific length of `bits`
    /// into the given `target`.
    ///
    /// The function uses the generic or 'turbofish' syntax to optimize the
    /// write call withing 8-bit boundaries from `Max8` to `Max64`. The intent
    /// is to use the minimum boundary that fits the selected use case.
    ///
    /// The padding of the variable must be clean as in all zeroes.
    ///
    /// The function might return an error once the internal buffer fills up
    /// and is flushed into the given target.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::fs::File;
    /// use variable_size_byte_writer::*;
    ///
    /// # fn f() -> std::io::Result<()> {
    /// let mut file = File::create("path")?;
    /// let mut writer = VariableSizeByteWriter::new(file);
    ///
    /// writer.write::<Max40>(0x71CFFABFF, 35)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn write<M>(&mut self, variable: u64, bits: u32) -> std::io::Result<()>
    where
        M: ByteSize,
    {
        if !self.can_insert::<M>() {
            self.flush_complete_bytes()?;
        }
        self.insert::<M>(variable, bits);
        Ok(())
    }

    /// Flushes the remaining internal buffer.
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
    /// let mut file = File::create("path")?;
    /// let mut writer = VariableSizeByteWriter::new(file);
    ///
    /// writer.write::<Max16>(0xAF, 9)?;
    /// writer.write::<Max16>(0x1A4, 11)?;
    /// writer.write::<Max16>(0x7B, 8)?;
    ///
    /// let mut padding = 0;
    /// writer.flush(&mut padding)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn flush(&mut self, padding: &mut u32) -> std::io::Result<()> {
        let bytes = self.all_bytes();
        *padding = self.padding();
        let mut written = 0;
        let result = self.write_range(0, bytes, &mut written);
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

    fn flush_complete_bytes(&mut self) -> std::io::Result<()> {
        let complete = self.complete_bytes();
        let mut written = 0;
        let result = self.write_range(0, complete, &mut written);
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

    fn write_range(&mut self, from: usize, to: usize, written: &mut usize) -> std::io::Result<()> {
        *written = 0;
        while from + *written < to {
            match self.target.write(&self.buf[from + *written..to]) {
                Ok(0) => return Err(Error::new(ErrorKind::WriteZero, "zero bytes written")),
                Ok(bytes) => *written += bytes,
                Err(ref err) if err.kind() == ErrorKind::Interrupted => {}
                Err(err) => return Err(err),
            }
        }
        Ok(())
    }

    #[inline]
    fn can_insert<M>(&mut self) -> bool
    where
        M: ByteSize,
    {
        let bytes = M::to_usize();
        let first = self.complete_bytes();
        first + bytes < self.buf.len()
    }

    #[inline]
    fn insert<M>(&mut self, variable: u64, bits: u32)
    where
        M: ByteSize,
    {
        let byte: usize = self.complete_bytes();
        let offset: u32 = self.partial_bits();

        let variable = variable.to_le();
        unsafe {
            *self.buf.get_unchecked_mut(byte as usize) |= (variable << offset) as u8;
            let mut variable = variable >> (8 - offset);

            M::full_unroll(|i| {
                *self.buf.get_unchecked_mut(byte + i + 1 as usize) = variable as u8;
                variable >>= 8;
            });
        }

        self.bits += bits;
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
    fn partial_byte(&self) -> u8 {
        if self.partial_bits() > 0 {
            self.buf[self.all_bytes() - 1]
        } else {
            0
        }
    }

    #[inline]
    fn padding(&self) -> u32 {
        (8 - self.partial_bits()) % 8
    }

    #[inline]
    fn erase_complete_bytes(&mut self) {
        let bytes = self.all_bytes();
        let possible_partial = self.partial_byte();
        self.buf[..bytes].iter_mut().for_each(|n| *n = 0);
        self.buf[0] = possible_partial;
        self.bits = self.partial_bits() as u32;
    }

    #[inline]
    fn erase_all_bytes(&mut self) {
        let bytes = self.all_bytes();
        self.buf[..bytes].iter_mut().for_each(|n| *n = 0);
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
}





#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let writer = VariableSizeByteWriter::new(Vec::new());
        assert_eq!(writer.bits, 0);
        assert_eq!(writer.buf.len(), 8192);
    }

    #[test]
    fn test_with_capacity() {
        let writer = VariableSizeByteWriter::with_capacity(Vec::new(), 1024);
        assert_eq!(writer.bits, 0);
        assert_eq!(writer.buf.len(), 1024);
    }

    #[test]
    fn test_complete_bytes() {
        let mut writer = VariableSizeByteWriter::new(Vec::new());
        writer.bits = 31;
        assert_eq!(writer.complete_bytes(), 3);
        writer.bits = 32;
        assert_eq!(writer.complete_bytes(), 4);
    }

    #[test]
    fn test_all_bytes() {
        let mut writer = VariableSizeByteWriter::new(Vec::new());
        writer.bits = 31;
        assert_eq!(writer.all_bytes(), 4);
        writer.bits = 32;
        assert_eq!(writer.all_bytes(), 4);
    }

    #[test]
    fn test_partial_bits() {
        let mut writer = VariableSizeByteWriter::new(Vec::new());
        writer.bits = 31;
        assert_eq!(writer.partial_bits(), 7);
        writer.bits = 32;
        assert_eq!(writer.partial_bits(), 0);
    }

    #[test]
    fn test_partial_byte() {
        let mut writer = VariableSizeByteWriter::new(Vec::new());
        writer.buf[4] = 0x1F;
        writer.bits = 37;
        assert_eq!(writer.partial_byte(), 0x1F);
        writer.bits = 40;
        assert_eq!(writer.partial_byte(), 0);
    }

    #[test]
    fn test_padding() {
        let mut writer = VariableSizeByteWriter::new(Vec::new());
        writer.bits = 33;
        assert_eq!(writer.padding(), 7);
        writer.bits = 32;
        assert_eq!(writer.padding(), 0);
    }

    #[test]
    fn test_erase_complete_bytes() {
        let mut writer = VariableSizeByteWriter::with_capacity(Vec::new(), 6);
        writer.buf[3] = 0xFF;
        writer.buf[4] = 0xF;
        writer.bits = 36;
        writer.erase_complete_bytes();
        assert_eq!(writer.bits, 4);
        assert_eq!(writer.buf[..], [0xF, 0, 0, 0, 0, 0]);

        let mut writer = VariableSizeByteWriter::with_capacity(Vec::new(), 6);
        writer.buf[3] = 0xFF;
        writer.bits = 32;
        writer.erase_complete_bytes();
        assert_eq!(writer.bits, 0);
        assert_eq!(writer.buf[..], [0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_erase_all_bytes() {
        let mut writer = VariableSizeByteWriter::with_capacity(Vec::new(), 6);
        writer.buf[3] = 0xFF;
        writer.buf[4] = 0xF;
        writer.bits = 36;
        writer.erase_all_bytes();
        assert_eq!(writer.bits, 0);
        assert_eq!(writer.buf[..], [0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_move_range_to_start() {
        let mut writer = VariableSizeByteWriter::with_capacity(Vec::new(), 6);
        writer.buf[4] = 0xAB;
        writer.buf[5] = 0xF;
        writer.bits = 44;
        writer.move_range_to_start(3, 6);
        assert_eq!(writer.bits, 20);
        assert_eq!(writer.buf[..], [0, 0xAB, 0xF, 0, 0, 0]);
    }

    #[test]
    fn test_flush_complete_bytes() {
        let mut writer = VariableSizeByteWriter::new(Vec::new());
        writer.buf[0] = 0xFF;
        writer.buf[1] = 0xA;
        writer.buf[2] = 0xAB;
        writer.buf[3] = 0xC;
        writer.bits = 28;
        writer.flush_complete_bytes().unwrap();
        assert_eq!(&writer.target[..3], [0xFF, 0xA, 0xAB]);
        assert_eq!(writer.bits, 4);
    }

    #[test]
    fn test_flush() {
        let mut writer = VariableSizeByteWriter::new(Vec::new());
        writer.buf[0] = 0xFF;
        writer.buf[1] = 0xA;
        writer.buf[2] = 0xAB;
        writer.buf[3] = 0xC;
        writer.bits = 28;
        let mut padding = 0;
        writer
            .flush(&mut padding)
            .unwrap();
        assert_eq!(&writer.target[..4], [0xFF, 0xA, 0xAB, 0xC]);
        assert_eq!(writer.bits, 0);
        assert_eq!(padding, 4);
    }

    #[test]
    fn test_write_range() {
        let mut writer = VariableSizeByteWriter::new(Vec::new());
        writer.buf[3] = 0xFF;
        writer.buf[4] = 0xF;
        writer.bits = 36;

        let mut written: usize = 0;
        writer.write_range(0, 4, &mut written).unwrap();
        assert_eq!(written, 4);
        assert_eq!(&writer.target[..4], [0, 0, 0, 0xFF]);

        let mut writer = VariableSizeByteWriter::new(Vec::new());
        writer.buf[3] = 0xFF;
        writer.buf[4] = 0xF;
        writer.bits = 36;

        let mut written: usize = 0;
        writer.write_range(2, 4, &mut written).unwrap();
        assert_eq!(written, 2);
        assert_eq!(&writer.target[..2], [0, 0xFF]);
    }

    #[test]
    fn test_write() {
        let mut writer = VariableSizeByteWriter::with_capacity(Vec::new(), 4);

        writer.write::<Max8>(0xA, 4).unwrap();
        assert_eq!(writer.buf[..], [0xA, 0, 0, 0]);
        assert_eq!(writer.bits, 4);
        assert_eq!(&writer.target[..], []);

        writer.write::<Max16>(0x7CB, 11).unwrap();
        assert_eq!(writer.buf[..], [0xBA, 0x7C, 0, 0]);
        assert_eq!(writer.bits, 15);
        assert_eq!(&writer.target[..], []);

        writer.write::<Max16>(0xFFFF, 16).unwrap();
        assert_eq!(writer.buf[..], [0xBA, 0xFC, 0xFF, 0x7F]);
        assert_eq!(writer.bits, 31);
        assert_eq!(&writer.target[..], []);

        writer.write::<Max8>(0xE1, 5).unwrap();
        assert_eq!(writer.buf[..], [0xFF, 0x70, 0, 0]);
        assert_eq!(writer.bits, 12);
        assert_eq!(&writer.target[..], [0xBA, 0xFC, 0xFF]);
    }

    #[test]
    fn test_can_insert() {
        let mut writer = VariableSizeByteWriter::with_capacity(Vec::new(), 6);
        writer.bits = 40;
        assert_eq!(writer.can_insert::<Max8>(), false);
        writer.bits = 39;
        assert_eq!(writer.can_insert::<Max8>(), true);
        writer.bits = 32;
        assert_eq!(writer.can_insert::<Max16>(), false);
        writer.bits = 31;
        assert_eq!(writer.can_insert::<Max16>(), true);
        writer.bits = 24;
        assert_eq!(writer.can_insert::<Max24>(), false);
        writer.bits = 23;
        assert_eq!(writer.can_insert::<Max24>(), true);
    }

    #[test]
    fn test_insert() {
        let mut writer = VariableSizeByteWriter::new(Vec::new());
        writer.insert::<Max8>(0xF, 4);
        assert_eq!(writer.buf[0..2], [0xF, 0]);
        assert_eq!(writer.bits, 4);

        writer.insert::<Max24>(0x77AFA, 20);
        assert_eq!(writer.buf[0..3], [0xAF, 0xAF, 0x77]);
        assert_eq!(writer.bits, 24);

        writer.insert::<Max16>(0x1BB, 9);
        assert_eq!(writer.buf[0..5], [0xAF, 0xAF, 0x77, 0xBB, 0x1]);
        assert_eq!(writer.bits, 33);
    }

    #[test]
    fn test_drop() {
        use std::fs::File;
        {
            let file = File::create("test_drop_temporary_file_buffer.temp").unwrap();
            let mut writer = VariableSizeByteWriter::new(file);
            writer.write::<Max8>(0xF, 4).unwrap();
            writer.write::<Max16>(0x7FF, 11).unwrap();
            assert_eq!(writer.buf[..4], [0xFF, 0x7F, 0, 0]);
        }
        let mut file = File::open("test_drop_temporary_file_buffer.temp").unwrap();
        let mut contents = vec![];
        file.read_to_end(&mut contents).unwrap();
        std::fs::remove_file("test_drop_temporary_file_buffer.temp").unwrap();
        assert_eq!(contents[..], [0xFF, 0x7F]);
    }
}
