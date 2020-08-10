# LibLoading Bindgen

[![Continuous integration](https://github.com/Michael-F-Bryan/libloading-bindgen/workflows/Continuous%20integration/badge.svg?branch=master)](https://github.com/Michael-F-Bryan/libloading-bindgen/actions)

([API Docs])

An extension to the [bindgen][bg] tool which generates code for loading native
libraries at runtime, instead of linking statically or dynamically.

## Getting Started

The easiest way to use this tool is via the `cargo libloading-binding` tool. You
can install it via `cargo`:

```console
cargo install --git https://github.com/Michael-F-Bryan/libloading-bindgen --bin cargo-libloading-bindgen
```

The workflow is very similar to that described in the [*Bindgen User Guide*][user-guide].
First, make sure you have a C header file containing the code you'd like to generate
bindings for:

```c
int smoke_test_add(int left, int right);
```

Next run the `cargo libloading-binding` program, using the `--whitelist-functions`
flag to filter just the functionality you want.

```console
cargo libloading-bindgen --whitelist-function 'smoke_test.*' -- /tmp/bindings.h
```

The generated bindings look something like this (piped through `rustfmt` for
clarity).

```rust
pub struct Bindings {
    /// Safety: We need to keep the library handle around because our vtable's
    /// pointers point into it.
    _library: ::libloading::Library,
    smoke_test_add: unsafe extern "C" fn(
        ::std::os::raw::c_int,
        ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int,
}

impl Bindings {
    pub unsafe fn load_from_path<P>(
        path: P,
    ) -> Result<Self, ::libloading::Error>
    where
        P: AsRef<::std::ffi::OsStr>,
    {
        let library = ::libloading::Library::new(path)?;
        let smoke_test_add = *library.get(b"smoke_test_add")?;
        Ok(Bindings {
            _library: library,
            smoke_test_add,
        })
    }
}

impl Bindings {
    pub unsafe fn smoke_test_add(
        &self,
        left: ::std::os::raw::c_int,
        right: ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int {
        (self.smoke_test_add)(left, right)
    }
}
```

It is recommended to add these bindings to revision control.

Alternatively, the `libloading-bindgen` crate can be used from a build script
to regenerate the bindings as part of the normal build process.

## License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE.md) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT.md) or
   http://opensource.org/licenses/MIT)

at your option.

It is recommended to always use [cargo-crev][crev] to verify the
trustworthiness of each of your dependencies, including this one.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.

The intent of this crate is to be free of soundness bugs. The developers will
do their best to avoid them, and welcome help in analysing and fixing them.

[API Docs]: https://michael-f-bryan.github.io/libloading-bindgen
[crev]: https://github.com/crev-dev/cargo-crev
[bg]: https://github.com/rust-lang/rust-bindgen
[user-guide]: https://rust-lang.github.io/rust-bindgen/
