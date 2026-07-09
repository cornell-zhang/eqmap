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
  input wire a;
  input wire b;
  input wire c;
  input wire d;
  input wire e;
  input wire f;
  input wire g;
  output wire y;

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
// CHECK:   a,
// CHECK:   b,
// CHECK:   c,
// CHECK:   d,
// CHECK:   e,
// CHECK:   f,
// CHECK:   g,
// CHECK:   y
// CHECK: );
// CHECK:   input wire a;
// CHECK:   input wire b;
// CHECK:   input wire c;
// CHECK:   input wire d;
// CHECK:   input wire e;
// CHECK:   input wire f;
// CHECK:   input wire g;
// CHECK:   output wire y;
// CHECK:   wire __0__;
// CHECK:   wire __1__;
// CHECK:   wire __2__;
// CHECK:   LUT3 #(
// CHECK:     .INIT(8'h96)
// CHECK:   ) __3__ (
// CHECK:     .I2(b),
// CHECK:     .I1(a),
// CHECK:     .I0(d),
// CHECK:     .O(__0__)
// CHECK:   );
// CHECK:   LUT3 #(
// CHECK:     .INIT(8'h17)
// CHECK:   ) __4__ (
// CHECK:     .I2(b),
// CHECK:     .I1(a),
// CHECK:     .I0(d),
// CHECK:     .O(__1__)
// CHECK:   );
// CHECK:   LUT6 #(
// CHECK:     .INIT(64'he8808000fffefee8)
// CHECK:   ) __5__ (
// CHECK:     .I5(__1__),
// CHECK:     .I4(c),
// CHECK:     .I3(e),
// CHECK:     .I2(g),
// CHECK:     .I1(f),
// CHECK:     .I0(__0__),
// CHECK:     .O(__2__)
// CHECK:   );
// CHECK:   assign y = __2__;
// CHECK: endmodule
