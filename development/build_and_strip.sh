#!/bin/bash

set -e

cd "$(dirname $0)"
cd ..

rm ./target/release/bindrs &>/dev/null || true
FULL_TOOLCHAIN="$(rustup toolchain list | grep default | awk '{print $1}' | cut -d '-' -f2-)"
cargo build --release

echo "Before Strip: $(ls -lh ./target/release/bindrs | awk '{print $5}')"
strip ./target/release/bindrs
echo " After Strip: $(ls -lh ./target/release/bindrs | awk '{print $5}')"

VERSION="$(./target/release/bindrs -V | awk '{print $2}')"
FILENAME="bindrs-$VERSION-$FULL_TOOLCHAIN.tar.gz"

cd ./target/release
tar -zcf $FILENAME ./bindrs
echo "GZipped File: $(ls -lh $FILENAME | awk '{print $5}')"

cd ../..
mkdir -p ./pkg
mv ./target/release/$FILENAME ./pkg/

echo "Built and zipped to ./pkg/$FILENAME"
