// RUN: parse-verilog %s 2>>/dev/null | FileCheck %s

module mux_4_1 (
    a,
    b,
    c,
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
  LUT3 #(
      .INIT(8'hCA)
  ) _2_ (
      .I0(tmp0),
      .I1(tmp1),
      .I2(s1),
      .O (y)
  );
  // CHECK: (LUT 202 s1 (LUT 202 s0 a b) (LUT 202 s0 c d))
endmodule
