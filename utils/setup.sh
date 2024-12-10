#!/bin/bash
cargo build --release --features default,$1,$2,$3,$4

chmod +x ./bin/*
chmod +x ./utils/*
export PATH=$PWD/target/release:$PWD/bin:$PWD/utils:$PATH

echo "SUCCESS"
