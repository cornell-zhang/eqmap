// RUN: cargo run --release --bin parse-verilog -- %s  | FileCheck %s

module muxf (
    a,
    b,
    s,
    y
);
  input a;
  wire a;
  input b;
  wire b;
  input s;
  wire s;
  output y;
  wire y;
  MUXF7 _4_ (
      .I0(b),
      .I1(a),
      .S (s),
      .O (y)
  );

  // CHECK: (MUX s a b)
endmodule
