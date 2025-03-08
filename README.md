![](https://github.com/matth2k/lut-synth/actions/workflows/rust.yml/badge.svg)

# E-Pack: Technology Mapping with E-Graphs

## Description

A Verilog-to-Verilog tool for superoptimizing FPGA netlists with E-Graphs

### Dependencies

#### Building

- Bash Shell (use WSL for Windows)
- [rustup](https://rustup.rs/)
  - Crates (these are fetched automatically)
    - [egg](https://docs.rs/egg/latest/egg/)
    - [bitvec](https://docs.rs/bitvec/latest/bitvec/)
    - [clap](https://docs.rs/clap/latest/clap/)
    - [indicatif](https://docs.rs/indicatif/latest/indicatif/)
    - [sv-parser](https://docs.rs/sv-parser/latest/sv_parser/)
    - [serde_json](https://docs.rs/serde_json/latest/serde_json/)
- ILP (only when using Cargo feature `exactness`)
  - [CBC Solver](https://github.com/coin-or/Cbc)

#### Development

- VSCode
  - [Rust Analyzer Extension](https://rust-analyzer.github.io/)
  - [VerilogHDL Extension](https://marketplace.visualstudio.com/items?itemName=mshr-h.VerilogHDL)
- RTL Tools
  - [Verible](https://github.com/chipsalliance/verible)
  - [Yosys](https://github.com/YosysHQ/yosys)

### Installing & Getting Started

First, install all the prerequisites for building. For basic functionally, you need Rust and Linux or Mac OS.

`cargo build`

`cargo run --release -- tests/verilog/mux_reg.v # Sanity check`

You can also try to synthesize your own verilog `my_file.v`:

`source utils/setup.sh # Add tools to path`

`lvv my_file.v`

Use `--help` to get an overview of all the options the compiler has:

```
$ epak --help
Technology Mapping Optimization with E-Graphs

Usage: epak [OPTIONS] [INPUT] [OUTPUT]

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
  -k, --k <K>                      Max fan in size for extracted LUTs [default: 6]
  -w, --reg-weight <REG_WEIGHT>    Ratio of register cost to LUT cost [default: 1]
  -t, --timeout <TIMEOUT>          Build/extraction timeout in seconds [default: 10]
  -s, --node-limit <NODE_LIMIT>    Maximum number of nodes in graph [default: 48000]
  -n, --iter-limit <ITER_LIMIT>    Maximum number of rewrite iterations [default: 32]
  -h, --help                       Print help
  -V, --version                    Print version
```

### Features

The project has three conditionally compiled features:

1. `egraph_fold` (should really not be used)
2. `exactness` (used for exact synthesis, requires cbc)
3. `cut_analysis` (tracks principle inputs used in cut of logic)

To build with these features enabled:

`cargo build --release --features exactness`

Or to get the tools in your PATH:

`source utils/setup.sh exactness`

### Docs

You can generate most of the documentation with `cargo doc`.

Here is a rough outline of the type system defined by `LutLang`:

`<LutLang> ::= <Program> | <Node> | BUS <Node> ... <Node>`

It is important to note that there is an implicit coversion from BUS types to Node types. The least significant bit is taken.
REG expressions trivially result in inconclusive. Sequential logic isn't fully supported yet.

```
<Node> ::= <Const> | x | <Input> | NOR <Node> <Node> | MUX <Node> <Node> <Node>
            | LUT <Program> <Node> ... <Node> | REG <Node> | ARG <u64> | CYCLE <Node>


<Const> ::= false | true // Base type is a bool

<Input> ::= <String> // Any string is parsed as an input variable

<Program> ::= <u64> // Can store a program for up to 6 bits
```
