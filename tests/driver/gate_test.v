// RUN: fam %s --assert-sat -n 40 -k 4 | FileCheck %s

module gate_test (
    a,
    b,
    c,
    d,
    e,
    f,
    g,
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
      .A(_00_),
      .B(_01_),
      .S(_03_),
      .Y(tmp0)
  );
  XOR2 _12_ (
      .A(c),
      .B(f),
      .Y(_04_)
  );
  XOR2 _12_ (
      .A(_04_),
      .B(tmp0),
      .Y(y)
  );

endmodule

// CHECK: module gate_test (
// CHECK:     b,
// CHECK:     e,
// CHECK:     d,
// CHECK:     g,
// CHECK:     a,
// CHECK:     f,
// CHECK:     c,
// CHECK:     y
// CHECK: );
// CHECK:   input b;
// CHECK:   wire b;
// CHECK:   input e;
// CHECK:   wire e;
// CHECK:   input d;
// CHECK:   wire d;
// CHECK:   input g;
// CHECK:   wire g;
// CHECK:   input a;
// CHECK:   wire a;
// CHECK:   input f;
// CHECK:   wire f;
// CHECK:   input c;
// CHECK:   wire c;
// CHECK:   output y;
// CHECK:   wire y;
// CHECK:   wire tmp6;
// CHECK:   wire tmp7;
// CHECK:   LUT2 #(
// CHECK:       .INIT(4'h1)
// CHECK:   ) __0__ (
// CHECK:       .I0(g),
// CHECK:       .I1(a),
// CHECK:       .O(tmp6)
// CHECK:   );
// CHECK:   LUT4 #(
// CHECK:       .INIT(16'h55c0)
// CHECK:   ) __1__ (
// CHECK:       .I0(b),
// CHECK:       .I1(e),
// CHECK:       .I2(d),
// CHECK:       .I3(tmp6),
// CHECK:       .O(tmp7)
// CHECK:   );
// CHECK:   LUT3 #(
// CHECK:       .INIT(8'h96)
// CHECK:   ) __2__ (
// CHECK:       .I0(tmp7),
// CHECK:       .I1(f),
// CHECK:       .I2(c),
// CHECK:       .O(y)
// CHECK:   );
// CHECK: endmodule
