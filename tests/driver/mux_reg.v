// RUN: eqmap_fpga %s --assert-sat -k 4 | FileCheck %s

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
// CHECK:     a,
// CHECK:     d,
// CHECK:     c,
// CHECK:     s0,
// CHECK:     s1,
// CHECK:     clk,
// CHECK:     y
// CHECK: );
// CHECK:   input b;
// CHECK:   wire b;
// CHECK:   input a;
// CHECK:   wire a;
// CHECK:   input d;
// CHECK:   wire d;
// CHECK:   input c;
// CHECK:   wire c;
// CHECK:   input s0;
// CHECK:   wire s0;
// CHECK:   input s1;
// CHECK:   wire s1;
// CHECK:   input clk;
// CHECK:   wire clk;
// CHECK:   output y;
// CHECK:   wire y;
// CHECK:   wire __0__;
// CHECK:   wire __1__;
// CHECK:   LUT4 #(
// CHECK:       .INIT(16'hf0ca)
// CHECK:   ) __2__ (
// CHECK:       .I0(d),
// CHECK:       .I1(c),
// CHECK:       .I2(s0),
// CHECK:       .I3(s1),
// CHECK:       .O(__0__)
// CHECK:   );
// CHECK:   LUT4 #(
// CHECK:       .INIT(16'hcaf0)
// CHECK:   ) __3__ (
// CHECK:       .I0(b),
// CHECK:       .I1(a),
// CHECK:       .I2(__0__),
// CHECK:       .I3(s1),
// CHECK:       .O(__1__)
// CHECK:   );
// CHECK:   FDRE #(
// CHECK:       .INIT(1'hx)
// CHECK:   ) __4__ (
// CHECK:       .C(clk),
// CHECK:       .CE(1'h1),
// CHECK:       .D(__1__),
// CHECK:       .R(1'h0),
// CHECK:       .Q(y)
// CHECK:   );
// CHECK: endmodule
