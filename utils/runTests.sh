#!/bin/bash
set -exo pipefail

# Optimization tests on LutLang IR
cargo run --release tests/lutlang/examples.txt -k 4 --no-verify | FileCheck tests/lutlang/examples.txt
cargo run --release tests/lutlang/bus.txt -k 4 -n 24 | FileCheck tests/lutlang/bus.txt
# Run again but with proofs
cargo run --release tests/lutlang/examples.txt -k 4 --no-verify -v | FileCheck tests/lutlang/examples.txt

# Verilog compilation tests
cargo run --release --bin parse-verilog -- tests/verilog/mux_4_1_synth.v 2>>/dev/null | FileCheck tests/verilog/mux_4_1_synth.v
cargo run --release --bin parse-verilog -- tests/verilog/mux_4_1_k3.v 2>>/dev/null | FileCheck tests/verilog/mux_4_1_k3.v
cargo run --release --bin parse-verilog -- tests/verilog/add_synth.v 2>>/dev/null | FileCheck tests/verilog/add_synth.v


echo "SUCCESS"
