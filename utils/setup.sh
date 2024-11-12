#!/bin/bash
cargo build
cargo build --release

chmod +x ./bin/*
export PATH=$PWD/target/release:$PWD/bin:$PATH

echo "SUCCESS"
