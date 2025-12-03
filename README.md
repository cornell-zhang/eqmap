![](https://github.com/cornell-zhang/eqmap/actions/workflows/rust.yml/badge.svg)
[![Docs](https://img.shields.io/badge/docs-github--pages-blue)](https://cornell-zhang.github.io/eqmap/)

# EqMap: FPGA LUT Technology Mapping w/ E-Graphs

EqMap is Verilog-to-Verilog tool that attempts to superoptimize FPGA technology mapping using E-Graphs. Our experiments show that equality saturation techniques can improve logic sut selection and ultimately produce smaller circuits than the commercial tools.

You might also want to check out the [docs](https://cornell-zhang.github.io/eqmap/) or the [ICCAD publication](https://github.com/cornell-zhang/eqmap/blob/main/eqmap_iccad.pdf).

## Getting Started

### Dependencies for Users

- [rustup](https://rustup.rs/)
  - Crates (fetched automatically)
    - [egg](https://docs.rs/egg/latest/egg/), [safety-net](https://docs.rs/safety-net/latest/safety_net/), [bitvec](https://docs.rs/bitvec/latest/bitvec/), [clap](https://docs.rs/clap/latest/clap/), [indicatif](https://docs.rs/indicatif/latest/indicatif/), [sv-parser](https://docs.rs/sv-parser/latest/sv_parser/), [serde_json](https://docs.rs/serde_json/latest/serde_json/)
- [Yosys 0.33](https://github.com/YosysHQ/yosys/releases/tag/yosys-0.33)
- *Optional* [CBC Solver](https://github.com/coin-or/Cbc)

### Dependencies for Devs

- VSCode Extensions
  - [Rust Analyzer Extension](https://rust-analyzer.github.io/)
  - [VerilogHDL Extension](https://marketplace.visualstudio.com/items?itemName=mshr-h.VerilogHDL)
- RTL Tools
  - [Verilator](https://github.com/verilator/verilator)
  - [Verible](https://github.com/chipsalliance/verible)

### Building the Tools

First, check the prerequisites for building. For basic functionality, you will need the Rust toolchain and a Yosys 0.33 install. Linux is preferred, but MacOS and WSL should work without much trouble.

`cargo build`

`cargo run --release -- tests/verilog/mux_reg.v # Sanity check`

### Bring Your Own RTL

You can also try to synthesize your own verilog module `my_file.v`, but it must confirm to a strict subset of Verilog. For example, the module must have a flat hierarchy and all top-level ports must be 1 bit signals.

`source utils/setup.sh # Add eqmap script to PATH`

`eqmap my_file.v`

Use `--help` to get an overview of all the options the compiler has:

```
$ eqmap --help
Technology Mapping Optimization with E-Graphs

Usage: eqmap_fpga [OPTIONS] [INPUT] [OUTPUT]

Arguments:
  [INPUT]   Verilog file to read from (or use stdin)
  [OUTPUT]  Verilog file to output to (or use stdout)

Options:
      --report <REPORT>            If provided, output a JSON file with result data
  -a, --assert-sat                 Return an error if the graph does not reach saturation
  -f, --no-verify                  Do not verify the functionality of the output
  -c, --no-canonicalize            Do not canonicalize the input into LUTs
  -d, --decomp                     Find new decompositions at runtime
      --disassemble <DISASSEMBLE>  Comma separated list of cell types to decompose into
  -r, --no-retime                  Do not use register retiming
  -v, --verbose                    Print explanations (generates a proof and runs slower)
      --min-depth                  Extract for minimum circuit depth
  -k, --k <K>                      Max fan in size allowed for extracted LUTs
  -w, --reg-weight <REG_WEIGHT>    Ratio of register cost to LUT cost
  -t, --timeout <TIMEOUT>          Build/extraction timeout in seconds
  -s, --node-limit <NODE_LIMIT>    Maximum number of nodes in graph
  -n, --iter-limit <ITER_LIMIT>    Maximum number of rewrite iterations
  -h, --help                       Print help
  -V, --version                    Print version
```

You will likely want to use the `--report <file>` flag to measure improvements in LUT count and circuit depth.

### Features

The project has three conditionally compiled features:

1. `egraph_fold` (deprecated)
1. `exactness` (used for ILP exact synthesis, requires [CBC](https://github.com/coin-or/Cbc))
1. `cut_analysis` (on by default)
1. `graph_dumps` (enables the serialization module and `--dump-graph` argument)

To build with any of these features enabled:

`source utils/setup.sh <feature>`

### Docs

You can generate most of the documentation with `cargo doc`.

### Citation

```bibtex
@INPROCEEDINGS{11240672,
  author={Hofmann, Matthew and Gokmen, Berk and Zhang, Zhiru},
  booktitle={2025 IEEE/ACM International Conference On Computer Aided Design (ICCAD)}, 
  title={EqMap: FPGA LUT Remapping using E-Graphs}, 
  year={2025},
  volume={},
  number={},
  pages={1-9},
  keywords={Runtime;Design automation;Heuristic algorithms;Circuits;Table lookup;Computational complexity;Field programmable gate arrays},
  doi={10.1109/ICCAD66269.2025.11240672}}
```
