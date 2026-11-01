# C API for resvg

## Build

```sh
cargo build --release
```

This will produce dynamic and static C libraries that can be found at `../target/release`.

## Header generation

The `resvg.h` is generated via [cbindgen](https://github.com/mozilla/cbindgen)
and then manually edited a bit.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/license/mit>)

at your option.

## Contribution

Contributions are welcome by pull request.
The [Rust code of conduct] applies.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be licensed as above, without any additional terms or conditions.

[Rust code of conduct]: https://rust-lang.org/policies/code-of-conduct/
