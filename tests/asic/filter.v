// RUN: eqmap_asic %s --filter "MUX2" -t 1 2>>/dev/null | FileCheck %s

module filter (
    a,
    b,
    y
);

  input a;
  input b;
  wire a;
  wire b;
  output y;
  wire y;

  OR _00_ (
      .A(a),
      .B(b),
      .Y(y)
  );

  // CHECK: MUX2_X1 #(
  // CHECK: ) __0__ (
  // CHECK:     .A(b),
  // CHECK:     .B(a),
  // CHECK:     .S(a),
  // CHECK:     .Z(y)
  // CHECK: );

endmodule
