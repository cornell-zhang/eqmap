// RUN: cargo run --release --bin parse-verilog -- %s 2>>/dev/null | FileCheck %s

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
  // CHECK: (LUT 16753531355601501928 (LUT 23 b a d) c e g f (LUT 150 b a d))
endmodule
