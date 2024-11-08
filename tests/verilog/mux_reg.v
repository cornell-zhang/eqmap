// RUN: parse-verilog %s 2>>/dev/null | FileCheck %s

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
  // CHECK: (LUT 202 (REG s1) (REG (LUT 202 s0 a b)) (REG (LUT 202 s0 c d)))
endmodule
