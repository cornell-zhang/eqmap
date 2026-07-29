// RUN: nl_opt %s -x -p remove-inv-pair,print-verilog | FileCheck %s

module inv_five (
    input  wire a,
    output wire y
);
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
  // CHECK:   input wire a,
  // CHECK:   output wire y
  // CHECK: );
  // CHECK:   INV
  // CHECK:     .A(a),
  // CHECK:     .ZN([[TMP:.*]])
  // CHECK:   );
  // CHECK:   assign y = [[TMP]]
  // CHECK: endmodule

endmodule
