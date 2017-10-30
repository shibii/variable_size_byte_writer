#![feature(test)]

extern crate test;
extern crate variable_size_byte_writer;

use test::Bencher;
use variable_size_byte_writer::*;

#[bench]
fn write_64_vec(bench: &mut Bencher) {
    let mut writer = VariableSizeByteWriter::new();
    let mut target = std::io::Cursor::new(vec![]);

    bench.iter(|| writer.write_64(&mut target, 0x1_ABAB_FFFF, 33));
}

#[bench]
fn write_64_file(bench: &mut Bencher) {
    let mut writer = VariableSizeByteWriter::new();
    let _res = std::fs::create_dir("benches/temp");
    let mut file = std::fs::File::create("benches/temp/write_64_file.temp").unwrap();
    bench.iter(|| writer.write_64(&mut file, 0x1_ABAB_FFFF, 33));
    std::fs::remove_file("benches/temp/write_64_file.temp").unwrap();
}

#[bench]
fn write_32_vec(bench: &mut Bencher) {
    let mut writer = VariableSizeByteWriter::new();
    let mut target = std::io::Cursor::new(vec![]);

    bench.iter(|| writer.write_32(&mut target, 0x7_F1F0, 21));
}

#[bench]
fn write_32_file(bench: &mut Bencher) {
    let mut writer = VariableSizeByteWriter::new();
    let _res = std::fs::create_dir("benches/temp");
    let mut file = std::fs::File::create("benches/temp/write_32_file.temp").unwrap();
    bench.iter(|| writer.write_32(&mut file, 0x7_F1F0, 21));
    std::fs::remove_file("benches/temp/write_32_file.temp").unwrap();
}

#[bench]
fn write_16_vec(bench: &mut Bencher) {
    let mut writer = VariableSizeByteWriter::new();
    let mut target = std::io::Cursor::new(vec![]);

    bench.iter(|| writer.write_16(&mut target, 0x1A, 9));
}

#[bench]
fn write_16_file(bench: &mut Bencher) {
    let mut writer = VariableSizeByteWriter::new();
    let _res = std::fs::create_dir("benches/temp");
    let mut file = std::fs::File::create("benches/temp/write_16_file.temp").unwrap();
    bench.iter(|| writer.write_16(&mut file, 0x1A, 9));
    std::fs::remove_file("benches/temp/write_16_file.temp").unwrap();
}

#[bench]
fn write_8_vec(bench: &mut Bencher) {
    let mut writer = VariableSizeByteWriter::new();
    let mut target = std::io::Cursor::new(vec![]);

    bench.iter(|| writer.write_8(&mut target, 0x1A, 7));
}

#[bench]
fn write_8_file(bench: &mut Bencher) {
    let mut writer = VariableSizeByteWriter::new();
    let _res = std::fs::create_dir("benches/temp");
    let mut file = std::fs::File::create("benches/temp/write_8_file.temp").unwrap();
    bench.iter(|| writer.write_8(&mut file, 0x1A, 7));
    std::fs::remove_file("benches/temp/write_8_file.temp").unwrap();
}

