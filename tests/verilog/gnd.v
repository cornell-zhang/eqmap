// RUN: cargo run --release --bin parse-verilog -- %s  | FileCheck %s

module gnd (
    d,
    y
);
  input d;
  wire d;
  wire tmp;
  output y;
  wire y;
  GND GND (.G(tmp));
  AND2 AND2 (
      .A(d),
      .B(tmp),
      .Y(y)
  );

  // CHECK: (AND d false)
endmodule
