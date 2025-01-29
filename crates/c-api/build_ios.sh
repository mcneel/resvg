#!/usr/bin/env bash

# get the latest rustup
rustup update

# cargo-lipo is used to create IOS build
cargo install cargo-lipo

# add the relevant platform targets
rustup target add aarch64-apple-ios x86_64-apple-ios

# always clean before a build
cargo clean

# build for Apple Silicon
cargo build --target=aarch64-apple-ios --release --lib

cp ../target/aarch64-apple-ios/release/libresvg_rhino.a libresvg_rhino_ios.a


# echo where this file should be copied
if [ -f "libresvg_rhino_ios.a" ]
then
echo "libresvg_rhino_ios.a created - this file should be added to rhino_ios.xcodeproj along with resvg.h"
fi
