// RUN: cargo run --release --bin parse-verilog -- --round-trip %s 2>>/dev/null | FileCheck %s

module mux_4_1 (
    a,
    b,
    c,
    d,
    s0,
    s1,
    y
);
  input a;
  wire a;
  input b;
  wire b;
  input c;
  wire c;
  input d;
  wire d;
  input s0;
  wire s0;
  input s1;
  wire s1;
  output y;
  wire y;
  LUT6 #(
      .INIT(64'hf0f0ccccff00aaaa)
  ) _0_ (
      .I0(d),
      .I1(c),
      .I2(a),
      .I3(b),
      .I4(s1),
      .I5(s0),
      .O (y)
  );

  // CHECK: input a;
  // CHECK: wire a;
  // CHECK: input b;
  // CHECK: wire b;
  // CHECK: input c;
  // CHECK: wire c;
  // CHECK: input d;
  // CHECK: wire d;
  // CHECK: input s0;
  // CHECK: wire s0;
  // CHECK: input s1;
  // CHECK: wire s1;
  // CHECK: output y;
  // CHECK: wire y;
  // CHECK: LUT6 #(
  // CHECK:     .INIT(64'hf0f0ccccff00aaaa)
  // CHECK: ) _0_ (
  // CHECK:     .I0(d),
  // CHECK:     .I1(c),
  // CHECK:     .I2(a),
  // CHECK:     .I3(b),
  // CHECK:     .I4(s1),
  // CHECK:     .I5(s0),
  // CHECK:     .O(y)
  // CHECK: );
endmodule
