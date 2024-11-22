// RUN: fam %s --assert-sat -n 40 | FileCheck %s

module gate_test (
    a,
    b,
    c,
    d,
    e,
    f,
    g,
    s0,
    s1,
    y
);
  wire _00_;
  wire _01_;
  wire _02_;
  wire _03_;
  wire _04_;
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
  input s0;
  wire s0;
  input s1;
  wire s1;
  wire tmp0;
  wire tmp1;
  output y;
  wire y;
  AND2 _05_ (
      .A(d),
      .B(e),
      .Y(_00_)
  );
  NOT _06_ (
      .A(b),
      .Y(_01_)
  );
  NOT _07_ (
      .A(_02_),
      .Y(_03_)
  );
  NOR2 _08_ (
      .A(a),
      .B(g),
      .Y(_02_)
  );
  MUX _09_ (
      .A(_03_),
      .B(_01_),
      .S(s0),
      .Y(tmp0)
  );
  MUX _10_ (
      .A(_04_),
      .B(_00_),
      .S(s0),
      .Y(tmp1)
  );
  MUX _11_ (
      .A(tmp1),
      .B(tmp0),
      .S(s1),
      .Y(y)
  );
  XOR2 _12_ (
      .A(c),
      .B(f),
      .Y(_04_)
  );

endmodule

// CHECK: module gate_test (
// CHECK:     b,
// CHECK:     g,
// CHECK:     a,
// CHECK:     s0,
// CHECK:     e,
// CHECK:     d,
// CHECK:     f,
// CHECK:     c,
// CHECK:     s1,
// CHECK:     y
// CHECK: );
// CHECK:   input b;
// CHECK:   wire b;
// CHECK:   input g;
// CHECK:   wire g;
// CHECK:   input a;
// CHECK:   wire a;
// CHECK:   input s0;
// CHECK:   wire s0;
// CHECK:   input e;
// CHECK:   wire e;
// CHECK:   input d;
// CHECK:   wire d;
// CHECK:   input f;
// CHECK:   wire f;
// CHECK:   input c;
// CHECK:   wire c;
// CHECK:   input s1;
// CHECK:   wire s1;
// CHECK:   output y;
// CHECK:   wire y;
// CHECK:   wire tmp5;
// CHECK:   wire tmp8;
// CHECK:   wire tmp11;
// CHECK:   LUT4 #(
// CHECK:       .INIT(16'hfc55)
// CHECK:   ) __0__ (
// CHECK:       .I0(b),
// CHECK:       .I1(g),
// CHECK:       .I2(a),
// CHECK:       .I3(s0),
// CHECK:       .O(tmp5)
// CHECK:   );
// CHECK:   LUT2 #(
// CHECK:       .INIT(4'h8)
// CHECK:   ) __1__ (
// CHECK:       .I0(e),
// CHECK:       .I1(d),
// CHECK:       .O(tmp8)
// CHECK:   );
// CHECK:   LUT4 #(
// CHECK:       .INIT(16'h3caa)
// CHECK:   ) __2__ (
// CHECK:       .I0(tmp8),
// CHECK:       .I1(f),
// CHECK:       .I2(c),
// CHECK:       .I3(s0),
// CHECK:       .O(tmp11)
// CHECK:   );
// CHECK:   LUT3 #(
// CHECK:       .INIT(8'hca)
// CHECK:   ) __3__ (
// CHECK:       .I0(tmp5),
// CHECK:       .I1(tmp11),
// CHECK:       .I2(s1),
// CHECK:       .O(y)
// CHECK:   );
// CHECK: endmodule
