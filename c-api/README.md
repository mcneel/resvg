# C API for resvg

## Build

```sh
cargo build --release
```

This will produce a dynamic C library that can be found at `../target/release`.

## macOS Universal binary

To build a macOS Universal binary that is compatible with Rhino, run:

```
./build_macos_universal.sh
```

and copy the *libresvg_rhino.dylib* to the rhino repository's */src4/rhino4/MacOS/Frameworks/* folder (overwriting the old one).  
