// RUN: parse-verilog %s 2>>/dev/null | FileCheck %s

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

  // CHECK: (MUX s1 (MUX s0 (XOR c f) (AND d e)) (MUX s0 (NOT (NOR a g)) (NOT b)))
endmodule
