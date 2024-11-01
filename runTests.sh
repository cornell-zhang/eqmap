#!/bin/bash
set -exo pipefail

cargo run --release tests/examples.txt -k 4 | FileCheck tests/examples.txt
# Run again but with proofs
cargo run --release tests/examples.txt -k 4 -v | FileCheck tests/examples.txt
echo "SUCCESS"
