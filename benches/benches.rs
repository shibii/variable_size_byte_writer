#![feature(test)]

extern crate test;
extern crate variable_size_byte_writer;

use test::Bencher;
use variable_size_byte_writer::*;

#[bench]
fn write_59bits_vec(bench: &mut Bencher) {
    let mut target = std::io::Cursor::new(vec![]);
    let mut writer = VariableSizeByteWriter::new(target);

    let bits = test::black_box(59);
    bench.iter(|| writer.write(0x7A2_5555_ABAB_FFFF, bits));
}

#[bench]
fn write_59bits_file(bench: &mut Bencher) {
    let _res = std::fs::create_dir("benches/temp");
    let file = std::fs::File::create("benches/temp/write_64_file.temp").unwrap();
    let mut writer = VariableSizeByteWriter::new(file);

    let bits = test::black_box(59);
    bench.iter(|| writer.write(0x7A2_5555_ABAB_FFFF, bits));

    std::fs::remove_file("benches/temp/write_64_file.temp").unwrap();
}

#[bench]
fn write_21bits_vec(bench: &mut Bencher) {
    let mut target = std::io::Cursor::new(vec![]);
    let mut writer = VariableSizeByteWriter::new(target);

    let bits = test::black_box(21);
    bench.iter(|| writer.write(0x7_F1F0, bits));
}

#[bench]
fn write_21bits_file(bench: &mut Bencher) {
    let _res = std::fs::create_dir("benches/temp");
    let file = std::fs::File::create("benches/temp/write_32_file.temp").unwrap();
    let mut writer = VariableSizeByteWriter::new(file);

    let bits = test::black_box(21);
    bench.iter(|| writer.write(0x7_F1F0, bits));

    std::fs::remove_file("benches/temp/write_32_file.temp").unwrap();
}

#[bench]
fn write_7bits_vec(bench: &mut Bencher) {
    let mut target = std::io::Cursor::new(vec![]);
    let mut writer = VariableSizeByteWriter::new(target);

    let bits = test::black_box(7);
    bench.iter(|| writer.write(0x1A, bits));
}

#[bench]
fn write_7bits_file(bench: &mut Bencher) {
    let _res = std::fs::create_dir("benches/temp");
    let file = std::fs::File::create("benches/temp/write_8_file.temp").unwrap();
    let mut writer = VariableSizeByteWriter::new(file);

    let bits = test::black_box(7);
    bench.iter(|| writer.write(0x1A, bits));

    std::fs::remove_file("benches/temp/write_8_file.temp").unwrap();
}