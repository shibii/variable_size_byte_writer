#[macro_use]
extern crate bencher;
extern crate variable_size_byte_writer;

use std::io::prelude::*;
use bencher::Bencher;
use variable_size_byte_writer::*;

fn write_32_vec(bench: &mut Bencher) {
    let mut writer = VariableSizeByteWriter::new(8192);
    let mut target = std::io::Cursor::new(vec![]);

    bench.iter(|| {
        writer.write_32(&mut target, 0x7F1F0, 21)
    });
}

fn write_32_file(bench: &mut Bencher) {
    let mut writer = VariableSizeByteWriter::new(8192);
    let mut file = std::fs::File::create("benches/temp/write_32_file.temp").unwrap();
    bench.iter(|| {
        writer.write_32(&mut file, 0x7F1F0, 21)
    });
    std::fs::remove_file("benches/temp/write_32_file.temp").unwrap();
}

benchmark_group!(benches, write_32_vec, write_32_file);
benchmark_main!(benches);