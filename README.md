# pylon-shimload

Rust wrapper of the Pylon libraries for Basler cameras using a pure C ABI shim
library.

The crate dynamically loads a C ABI shim library at runtime, which in turn loads
the Pylon library. This allows the crate to be used without linking to the Pylon
library at compile time, and also allows it to be used with different versions
of the Pylon library without recompilation.

The main API entry points are:

- Top-level helpers like `create_first_device`, `enumerate_devices`, and `version`
- `runtime::init()` and `runtime::RuntimeGuard` for explicit runtime lifecycle control
- `InstantCamera` to open a device and grab images
- `NodeMap` and the typed node wrappers to access GenICam parameters
- `GrabResult` to inspect acquired image buffers

## Compatibility

This crate is derived from [pylon-cxx](https://crates.io/crates/pylon-cxx),
which is a more direct wrapper of the C++ API. The API is mostly the same, but
some differences exist due to the C ABI shim. Porting code from pylon-cxx should
be straightforward.

## Platform support

Linux (x86_64) and macOS (aarch64) are tested. Windows code exists in the crate,
but is not currently tested.

## Quick start

The smallest useful setup check is:

```bash
cargo run --example show-version
```

To talk to a camera synchronously:

```bash
cargo run --example grab
```

Examples included in this repository:

- `show-version`: print the crate and loaded Pylon version
- `grab`: grab images synchronously
- `async-grab`: grab images as a tokio stream
- `show-properties`: list device properties
- `show-pixel-formats`: inspect available pixel formats
- `feature-persistence`: save and restore node-map settings
- `chunk-data`: inspect chunk data from acquired images
- `reset-all-devices`: execute a reset command on discovered devices

## async stream with tokio

Enable async stream support using tokio with the cargo feature `stream`.
The included async example also needs a multi-threaded tokio runtime:

```bash
cargo run --example async-grab --features stream,tokio/rt-multi-thread
```

## Downloading a shim

Shims for different platforms and Pylon versions can be found at
https://strawlab.org/assets/libpylon-cabi/precompiled/.

### Linux (x86_64)

```bash
SHIM_FILENAME="libpylon-cabi-v1-linux-x86_64-pylon_7.3.0.27189.so"
SHIM_URL="https://strawlab.org/assets/libpylon-cabi/precompiled/${SHIM_FILENAME}"
curl --fail -L -o libpylon-cabi.so "${SHIM_URL}"
export "PYLON_CABI=$(pwd)/libpylon-cabi.so"
```

### macOS (aarch64)

```bash
SHIM_FILENAME="libpylon-cabi-v1-macos-aarch64-pylon_7.3.1.9.dylib"
SHIM_URL="https://strawlab.org/assets/libpylon-cabi/precompiled/${SHIM_FILENAME}"
curl --fail -L -o libpylon-cabi.dylib "${SHIM_URL}"
export "PYLON_CABI=$(pwd)/libpylon-cabi.dylib"
```

## Compiling a shim

The source code for the C ABI shim library is in the `libpylon-cabi` directory. It
can be built independently of this Rust crate, and the resulting shared library
can be used with this crate.

Building locally requires the Basler Pylon SDK to be installed. The default
build paths are:

- Linux: `/opt/pylon`
- macOS: `/Library/Frameworks/pylon.framework`

Build the shim with:

```bash
make -C libpylon-cabi
```

You can override the Linux SDK location, for example:

```bash
make -C libpylon-cabi PYLON_ROOT=/opt/pylon
```

### Reproducible build of the shim

Reproducible Linux builds of the shim can be done with Docker. See
`libpylon-cabi/Dockerfile` for details.

## Running

The location of the C ABI shim library can be specified to this crate using the
`PYLON_CABI` environment variable. If this variable is not set, the crate will
look for the shim library in the standard system library locations.

```bash
cargo run --example grab
```

### On macOS

On macOS, building works, but the Pylon framework libraries must still be found
at runtime. If they are not already discoverable on your system, set:

    export DYLD_LIBRARY_PATH="/Library/Frameworks/pylon.framework/Versions/A/Libraries/"

An article like
https://jorgen.tjer.no/post/2014/05/20/dt-rpath-ld-and-at-rpath-dyld/ which
describes approaches that may avoid relying on `DYLD_LIBRARY_PATH`.

## Camera emulation

See [Basler's documentation](https://docs.baslerweb.com/camera-emulation). This can
simulate different frame rates, failures, etc.

```text
# on bash (e.g. linux)
export PYLON_CAMEMU=2
```

```text
# in Windows Powershell
$Env:PYLON_CAMEMU=2
```

## Code of conduct

Anyone who interacts with this software in any space, including but not limited
to this GitHub repository, must follow our [code of
conduct](code_of_conduct.md).

## License

This crate is Copyright (C) 2020 Andrew Straw <strawman@astraw.com>.

Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
https://www.apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
https://opensource.org/licenses/MIT>, at your option. This file may not be
copied, modified, or distributed except according to those terms.

Note that this license only covers this Rust crate. The underlying Pylon library
has different license terms.
