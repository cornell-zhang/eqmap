// RUN: fam %s --assert-sat --max-depth | FileCheck %s

module mux_reg (
    a,
    b,
    c,
    clk,
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
  input clk;
  wire clk;
  input d;
  wire d;
  input s0;
  wire s0;
  input s1;
  wire s1;
  output y;
  wire y;
  wire tmp0;
  wire tmp1;
  wire tmp0_r;
  wire tmp1_r;
  wire s1_r;
  LUT3 #(
      .INIT(8'hCA)
  ) _0_ (
      .I0(b),
      .I1(a),
      .I2(s0),
      .O (tmp1)
  );
  LUT3 #(
      .INIT(8'hCA)
  ) _1_ (
      .I0(d),
      .I1(c),
      .I2(s0),
      .O (tmp0)
  );
  FDRE #(
      .INIT(1'hx)
  ) _pipe_tmp0_ (
      .C (clk),
      .CE(1'h1),
      .D (tmp0),
      .Q (tmp0_r),
      .R (1'h0)
  );
  FDRE #(
      .INIT(1'hx)
  ) _pipe_tmp1_ (
      .C (clk),
      .CE(1'h1),
      .D (tmp1),
      .Q (tmp1_r),
      .R (1'h0)
  );
  FDRE #(
      .INIT(1'hx)
  ) _pipe_s1_ (
      .C (clk),
      .CE(1'h1),
      .D (s1),
      .Q (s1_r),
      .R (1'h0)
  );
  LUT3 #(
      .INIT(8'hCA)
  ) _2_ (
      .I0(tmp0_r),
      .I1(tmp1_r),
      .I2(s1_r),
      .O (y)
  );
endmodule

// CHECK: module mux_reg (
// CHECK:     b,
// CHECK:     clk,
// CHECK:     a,
// CHECK:     s0,
// CHECK:     d,
// CHECK:     c,
// CHECK:     s1,
// CHECK:     y
// CHECK: );
// CHECK:   input b;
// CHECK:   wire b;
// CHECK:   input clk;
// CHECK:   wire clk;
// CHECK:   input a;
// CHECK:   wire a;
// CHECK:   input s0;
// CHECK:   wire s0;
// CHECK:   input d;
// CHECK:   wire d;
// CHECK:   input c;
// CHECK:   wire c;
// CHECK:   input s1;
// CHECK:   wire s1;
// CHECK:   output y;
// CHECK:   wire y;
// CHECK:   wire tmp2;
// CHECK:   wire tmp4;
// CHECK:   wire tmp6;
// CHECK:   wire tmp8;
// CHECK:   wire tmp10;
// CHECK:   wire tmp11;
// CHECK:   wire tmp13;
// CHECK:   FDRE #(
// CHECK:       .INIT(1'hx)
// CHECK:   ) __0__ (
// CHECK:       .C(clk),
// CHECK:       .CE(1'h1),
// CHECK:       .D(b),
// CHECK:       .R(1'h0),
// CHECK:       .Q(tmp2)
// CHECK:   );
// CHECK:   FDRE #(
// CHECK:       .INIT(1'hx)
// CHECK:   ) __1__ (
// CHECK:       .C(clk),
// CHECK:       .CE(1'h1),
// CHECK:       .D(a),
// CHECK:       .R(1'h0),
// CHECK:       .Q(tmp4)
// CHECK:   );
// CHECK:   FDRE #(
// CHECK:       .INIT(1'hx)
// CHECK:   ) __2__ (
// CHECK:       .C(clk),
// CHECK:       .CE(1'h1),
// CHECK:       .D(s0),
// CHECK:       .R(1'h0),
// CHECK:       .Q(tmp6)
// CHECK:   );
// CHECK:   FDRE #(
// CHECK:       .INIT(1'hx)
// CHECK:   ) __3__ (
// CHECK:       .C(clk),
// CHECK:       .CE(1'h1),
// CHECK:       .D(d),
// CHECK:       .R(1'h0),
// CHECK:       .Q(tmp8)
// CHECK:   );
// CHECK:   FDRE #(
// CHECK:       .INIT(1'hx)
// CHECK:   ) __4__ (
// CHECK:       .C(clk),
// CHECK:       .CE(1'h1),
// CHECK:       .D(c),
// CHECK:       .R(1'h0),
// CHECK:       .Q(tmp10)
// CHECK:   );
// CHECK:   LUT3 #(
// CHECK:       .INIT(8'hca)
// CHECK:   ) __5__ (
// CHECK:       .I0(tmp8),
// CHECK:       .I1(tmp10),
// CHECK:       .I2(tmp6),
// CHECK:       .O(tmp11)
// CHECK:   );
// CHECK:   FDRE #(
// CHECK:       .INIT(1'hx)
// CHECK:   ) __6__ (
// CHECK:       .C(clk),
// CHECK:       .CE(1'h1),
// CHECK:       .D(s1),
// CHECK:       .R(1'h0),
// CHECK:       .Q(tmp13)
// CHECK:   );
// CHECK:   LUT5 #(
// CHECK:       .INIT(32'hcacaff00)
// CHECK:   ) __7__ (
// CHECK:       .I0(tmp2),
// CHECK:       .I1(tmp4),
// CHECK:       .I2(tmp6),
// CHECK:       .I3(tmp11),
// CHECK:       .I4(tmp13),
// CHECK:       .O(y)
// CHECK:   );
// CHECK: endmodule
