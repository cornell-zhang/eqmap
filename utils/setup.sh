#!/bin/bash
cargo build
cargo build --release

chmod +x ./bin/*
chmod +x ./utils/*
export PATH=$PWD/target/release:$PWD/bin:$PWD/utils:$PATH

echo "SUCCESS"
