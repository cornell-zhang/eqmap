// RUN: eqmap_fpga %s --assert-sat | FileCheck %s

module dropped_input (
    a,
    b,
    c,
    y
);
  input wire a;
  input wire b;
  input wire c;
  output wire y;
  AND _0_ (
      .A(a),
      .B(b),
      .Y(y)
  );

endmodule

// CHECK: module dropped_input (
// CHECK:   a,
// CHECK:   b,
// CHECK:   c,
// CHECK:   y
// CHECK: );
// CHECK:   input wire a;
// CHECK:   input wire b;
// CHECK:   input wire c;
// CHECK:   output wire y;
// CHECK:   wire __0__;
// CHECK:   LUT2 #(
// CHECK:     .INIT(4'h8)
// CHECK:   ) __1__ (
// CHECK:     .I1(a),
// CHECK:     .I0(b),
// CHECK:     .O(__0__)
// CHECK:   );
// CHECK:   assign y = __0__;
// CHECK: endmodule
