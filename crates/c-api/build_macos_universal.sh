#!/usr/bin/env bash

# get the latest rustup
rustup update

# add the relevant platform targets
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin

# always clean before a build
cargo clean

# Target the same version of macOS that we target in Rhino for Mac
export MACOSX_DEPLOYMENT_TARGET=12.4

# build for Intel
cargo build --target=x86_64-apple-darwin --release

# build for Apple Silicon
cargo build --target=aarch64-apple-darwin --release

cp ../target/x86_64-apple-darwin/release/libresvg_rhino.dylib libresvg_rhino_x86_64.dylib
cp ../target/aarch64-apple-darwin/release/libresvg_rhino.dylib libresvg_rhino_arm64.dylib

# Fix the rpaths
install_name_tool -id @rpath/libresvg_rhino.dylib libresvg_rhino_x86_64.dylib
install_name_tool -id @rpath/libresvg_rhino.dylib libresvg_rhino_arm64.dylib

# lipo the libraries together
lipo libresvg_rhino_x86_64.dylib libresvg_rhino_arm64.dylib -output libresvg_rhino.dylib -create

# echo where this file should be copied
if [ -f "libresvg_rhino.dylib" ]
then
rm libresvg_rhino_x86_64.dylib
rm libresvg_rhino_arm64.dylib
file libresvg_rhino.dylib
echo "libresvg_rhino.dylib created - this file should be copied to src4/rhino4/MacOS/Frameworks (overwriting the older version)"
fi
