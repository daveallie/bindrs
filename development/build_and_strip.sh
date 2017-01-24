#!/bin/bash

cd "$(dirname $0)"
cd ..

rm target/release/bindrs &>/dev/null
cargo build --release
echo "Before Strip: $(ls -lh target/release/bindrs | awk '{print $5}')"
strip target/release/bindrs
echo " After Strip: $(ls -lh target/release/bindrs | awk '{print $5}')"
