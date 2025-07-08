// RUN: eqmap_fpga %s --assert-sat | FileCheck %s

module add (
    a,
    b,
    c,
    d,
    e,
    f,
    g,
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
  input e;
  wire e;
  input f;
  wire f;
  input g;
  wire g;
  output y;
  wire y;

  // wire [2:0] sum = {2'b0, a} + {2'b0, b} + {2'b0, c} + {2'b0, d} + {2'b0, e} + {2'b0, f} + {2'b0, g};
  // assign y = sum[2];

  wire tmp0;
  wire tmp1;
  LUT6 #(
      .INIT(64'he8808000fffefee8)
  ) _1_ (
      .I0(tmp0),
      .I1(f),
      .I2(g),
      .I3(e),
      .I4(c),
      .I5(tmp1),
      .O (y)
  );
  LUT3 #(
      .INIT(8'h17)
  ) _2_ (
      .I0(d),
      .I1(a),
      .I2(b),
      .O (tmp1)
  );
  LUT3 #(
      .INIT(8'h96)
  ) _3_ (
      .I0(d),
      .I1(a),
      .I2(b),
      .O (tmp0)
  );

endmodule

// CHECK: module add (
// CHECK:     d,
// CHECK:     a,
// CHECK:     b,
// CHECK:     f,
// CHECK:     g,
// CHECK:     e,
// CHECK:     c,
// CHECK:     y
// CHECK: );
// CHECK:   input d;
// CHECK:   wire d;
// CHECK:   input a;
// CHECK:   wire a;
// CHECK:   input b;
// CHECK:   wire b;
// CHECK:   input f;
// CHECK:   wire f;
// CHECK:   input g;
// CHECK:   wire g;
// CHECK:   input e;
// CHECK:   wire e;
// CHECK:   input c;
// CHECK:   wire c;
// CHECK:   output y;
// CHECK:   wire y;
// CHECK:   wire __0__;
// CHECK:   wire __1__;
// CHECK:   LUT3 #(
// CHECK:       .INIT(8'h96)
// CHECK:   ) __2__ (
// CHECK:       .I0(d),
// CHECK:       .I1(a),
// CHECK:       .I2(b),
// CHECK:       .O(__0__)
// CHECK:   );
// CHECK:   LUT3 #(
// CHECK:       .INIT(8'h17)
// CHECK:   ) __3__ (
// CHECK:       .I0(d),
// CHECK:       .I1(a),
// CHECK:       .I2(b),
// CHECK:       .O(__1__)
// CHECK:   );
// CHECK:   LUT6 #(
// CHECK:       .INIT(64'he8808000fffefee8)
// CHECK:   ) __4__ (
// CHECK:       .I0(__0__),
// CHECK:       .I1(f),
// CHECK:       .I2(g),
// CHECK:       .I3(e),
// CHECK:       .I4(c),
// CHECK:       .I5(__1__),
// CHECK:       .O(y)
// CHECK:   );
// CHECK: endmodule
