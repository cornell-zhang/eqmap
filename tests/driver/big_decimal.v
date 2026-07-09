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
  input wire a;
  input wire b;
  input wire c;
  input wire d;
  input wire s0;
  input wire s1;
  output wire y;
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
// CHECK:   a,
// CHECK:   b,
// CHECK:   c,
// CHECK:   d,
// CHECK:   s0,
// CHECK:   s1,
// CHECK:   y
// CHECK: );
// CHECK:   input wire a;
// CHECK:   input wire b;
// CHECK:   input wire c;
// CHECK:   input wire d;
// CHECK:   input wire s0;
// CHECK:   input wire s1;
// CHECK:   output wire y;
// CHECK:   wire __0__;
// CHECK:   wire __1__;
// CHECK:   LUT4 #(
// CHECK:     .INIT(16'hf0ca)
// CHECK:   ) __2__ (
// CHECK:     .I3(s0),
// CHECK:     .I2(s1),
// CHECK:     .I1(b),
// CHECK:     .I0(d),
// CHECK:     .O(__0__)
// CHECK:   );
// CHECK:   LUT4 #(
// CHECK:     .INIT(16'hcaf0)
// CHECK:   ) __3__ (
// CHECK:     .I3(s0),
// CHECK:     .I2(__0__),
// CHECK:     .I1(a),
// CHECK:     .I0(c),
// CHECK:     .O(__1__)
// CHECK:   );
// CHECK:   assign y = __1__;
// CHECK: endmodule
