#!/bin/bash
set -exo pipefail

cargo build
cargo build --release

PATH=$PWD/target/release:$PWD/bin:$PATH python3 utils/test-runner.py tests -j 2

echo "SUCCESS"
