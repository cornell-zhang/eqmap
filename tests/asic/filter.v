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

  // CHECK: MUX2_X1 __1__ (
  // CHECK:     .S(b),
  // CHECK:     .B(b),
  // CHECK:     .A(a),
  // CHECK:     .Z(__0__)
  // CHECK: );
  // CHECK: assign y = __0__;

endmodule
