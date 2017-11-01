# variable_size_byte_writer

A Rust crate for writing variable-size bytes into io::Write traited targets.

Writes are internally buffered and so the usage of any additional buffering such as std::io::BufWriter is not recommended.

# Usage:

Writing some unconventionally sized bytes into `Vec<u8>`:

```
use variable_size_byte_writer::*;

let mut target = Vec::new();
let mut writer = VariableSizeByteWriter::new(target);
let bytes = [(0x3F, 6),(0x1AFF, 13),(0x7, 3)];

bytes
    .iter()
    .for_each(|&(byte, bits)|
        writer.write(byte, bits).unwrap()
    );
```

Writing a series of 7bit bytes into a file, manually
flushing the internal buffer and capturing the
required bits to pad the last byte:

```
use std::fs::File;
use variable_size_byte_writer::*;

let mut file = File::create("path").unwrap();
let mut writer = VariableSizeByteWriter::new(file);

for variable in 0..0x8F {
    writer.write(variable, 7).unwrap();
}

let mut padding = 0;
writer.flush(&mut padding).unwrap();
```











## Writing some unconventionally sized bytes into Vec<u8>

``` rust
use variable_size_byte_writer::*;

let mut target = Vec::new();
let mut writer = VariableSizeByteWriter::new(target);
let bytes = [(0x3F, 6),(0x1AFF, 13),(0x7, 3)];

bytes
    .iter()
    .for_each(|&(byte, bits)|
        writer.write(&mut target, byte, bits).unwrap()
    );
```

## Writing a series of 7bit bytes into a file

``` rust
use std::fs::File;
use variable_size_byte_writer::*;

let mut writer = VariableSizeByteWriter::new();
let mut file = File::create("path").unwrap();

for variable in 0..0x8F {
    writer.write(&mut file, variable, 7).unwrap();
}

let mut padding = 0;
writer
    .flush_all_bytes(&mut file, &mut padding)
    .unwrap();
```

# License
variable_size_byte_writer is distributed under the terms of MIT licence.