![](https://github.com/matth2k/eqmap/actions/workflows/rust.yml/badge.svg)

# EqMap: FPGA LUT Technology Mapping w/ E-Graphs

## Description

A Verilog-to-Verilog tool for superoptimizing FPGA netlists with E-Graphs

### Dependencies

#### To Build

- Bash Shell (use WSL for Windows)
- [rustup](https://rustup.rs/)
  - Crates (these are fetched automatically)
    - [egg](https://docs.rs/egg/latest/egg/)
    - [bitvec](https://docs.rs/bitvec/latest/bitvec/)
    - [clap](https://docs.rs/clap/latest/clap/)
    - [indicatif](https://docs.rs/indicatif/latest/indicatif/)
    - [sv-parser](https://docs.rs/sv-parser/latest/sv_parser/)
    - [serde_json](https://docs.rs/serde_json/latest/serde_json/)
- [Yosys 0.33](https://github.com/YosysHQ/yosys)
- *Optional* ILP
  - [CBC Solver](https://github.com/coin-or/Cbc)

#### For Development

- VSCode
  - [Rust Analyzer Extension](https://rust-analyzer.github.io/)
  - [VerilogHDL Extension](https://marketplace.visualstudio.com/items?itemName=mshr-h.VerilogHDL)
- RTL Tools
  - [Verilator](https://github.com/verilator/verilator)
  - [Verible](https://github.com/chipsalliance/verible)

### Installing & Getting Started

First, check the prerequisites for building. For basic functionality, you will need the Rust toolchain, a Yosys 0.33 install. Linux is preferred, but MacOS and WSL should work without much trouble.

`cargo build`

`cargo run --release -- tests/verilog/mux_reg.v # Sanity check`

You can also try to synthesize your own verilog module `my_file.v` as long as it has no multi-bit signals:

`source utils/setup.sh # Add tools to path`

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

### Features

The project has three conditionally compiled features:

1. `egraph_fold` (should really not be used)
2. `exactness` (used for exact synthesis, requires cbc)
3. `cut_analysis` (tracks principle inputs used in cut of logic)
4. `graph_dumps` (enables the serialization module and `--dump-graph` argument)

To build with these features enabled:

`cargo build --release --features exactness`

Or to get the tools in your PATH:

`source utils/setup.sh exactness`

### Docs

You can generate most of the documentation with `cargo doc`.

Here is a rough outline of the grammar defined by `LutLang`:

`<LutLang> ::= <Program> | <Node> | BUS <Node> ... <Node>`

REG expressions do not work with verification, and sequential logic isn't fully supported yet.

```
<Node> ::= <Const> | x | <Input> | NOR <Node> <Node> | MUX <Node> <Node> <Node>
            | LUT <Program> <Node> ... <Node> | REG <Node> | ARG <u64> | CYCLE <Node>


<Const> ::= false | true // Base type is a bool

<Input> ::= <String> // Any string is parsed as an input variable

<Program> ::= <u64> // Can store a program for up to 6 bits
```
