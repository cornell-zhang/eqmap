// RUN: eqmap_fpga %s --assert-sat -k 4 | FileCheck %s

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
      .INIT(64'd17361601744336890538)
  ) _0_ (
      .I0(d),
      .I1(c),
      .I2(a),
      .I3(b),
      .I4(s1),
      .I5(s0),
      .O (y)
  );
endmodule

// CHECK: module mux_4_1 (
// CHECK:     c,
// CHECK:     a,
// CHECK:     d,
// CHECK:     b,
// CHECK:     s1,
// CHECK:     s0,
// CHECK:     y
// CHECK: );
// CHECK:   input c;
// CHECK:   wire c;
// CHECK:   input a;
// CHECK:   wire a;
// CHECK:   input d;
// CHECK:   wire d;
// CHECK:   input b;
// CHECK:   wire b;
// CHECK:   input s1;
// CHECK:   wire s1;
// CHECK:   input s0;
// CHECK:   wire s0;
// CHECK:   output y;
// CHECK:   wire y;
// CHECK:   wire __0__;
// CHECK:   LUT4 #(
// CHECK:       .INIT(16'hf0ca)
// CHECK:   ) __1__ (
// CHECK:       .I0(d),
// CHECK:       .I1(b),
// CHECK:       .I2(s1),
// CHECK:       .I3(s0),
// CHECK:       .O(__0__)
// CHECK:   );
// CHECK:   LUT4 #(
// CHECK:       .INIT(16'hcaf0)
// CHECK:   ) __2__ (
// CHECK:       .I0(c),
// CHECK:       .I1(a),
// CHECK:       .I2(__0__),
// CHECK:       .I3(s0),
// CHECK:       .O(y)
// CHECK:   );
// CHECK: endmodule
