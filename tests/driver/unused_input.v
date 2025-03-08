// RUN: epak %s --assert-sat | FileCheck %s

module dropped_input (
    a,
    b,
    c,
    y
);
  input a;
  wire a;
  input b;
  wire b;
  input c;
  wire c;
  output y;
  wire y;
  AND2 _0_ (
      .A(a),
      .B(b),
      .Y(y)
  );

endmodule

// CHECK: module dropped_input (
// CHECK:     b,
// CHECK:     a,
// CHECK:     c,
// CHECK:     y
// CHECK: );
// CHECK:   input b;
// CHECK:   wire b;
// CHECK:   input a;
// CHECK:   wire a;
// CHECK:   input c;
// CHECK:   wire c;
// CHECK:   output y;
// CHECK:   wire y;
// CHECK:   LUT2 #(
// CHECK:       .INIT(4'h8)
// CHECK:   ) __0__ (
// CHECK:       .I0(b),
// CHECK:       .I1(a),
// CHECK:       .O(y)
// CHECK:   );
// CHECK: endmodule
