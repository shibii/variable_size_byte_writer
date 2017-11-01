# variable_size_byte_writer

[A Rust crate](https://crates.io/crates/variable_size_byte_writer) for writing variable-size bytes into `io::Write` traited targets.

Writes are internally buffered and so the usage of any additional buffering such as `std::io::BufWriter` is not recommended.

# License
variable_size_byte_writer is distributed under the terms of MIT licence.