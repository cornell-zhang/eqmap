// RUN: nl_opt %s -x -p remove-inv-pair,print-verilog | FileCheck %s

module inv_five (
    a,
    y
);
  input a;
  wire a;
  output y;
  wire y;
  wire tmp0;
  wire tmp1;
  wire tmp2;
  wire tmp3;

  INV _0_ (
      .A (a),
      .ZN(tmp0)
  );

  INV _1_ (
      .A (tmp0),
      .ZN(tmp1)
  );

  INV _2_ (
      .A (tmp1),
      .ZN(tmp2)
  );

  INV _3_ (
      .A (tmp2),
      .ZN(tmp3)
  );

  INV _4_ (
      .A (tmp3),
      .ZN(y)
  );

  // CHECK: module inv_five (
  // CHECK:   a,
  // CHECK:   y
  // CHECK: );
  // CHECK:   input a;
  // CHECK:   wire a;
  // CHECK:   output y;
  // CHECK:   wire y;
  // CHECK:   INV
  // CHECK:     .A(a),
  // CHECK:     .ZN([[TMP:.*]])
  // CHECK:   );
  // CHECK:   assign y = [[TMP]]
  // CHECK: endmodule

endmodule
