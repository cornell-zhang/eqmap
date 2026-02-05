// RUN: eqmap_fpga %s --disassemble NOR,INV,AND -s 140000 2>>/dev/null | FileCheck %s

module mux_4_1 (
    b,
    a,
    d,
    c,
    s0,
    s1,
    y
);
  input b;
  wire b;
  input a;
  wire a;
  input d;
  wire d;
  input c;
  wire c;
  input s0;
  wire s0;
  input s1;
  wire s1;
  output y;
  wire y;
  wire tmp7;
  LUT4 #(
      .INIT(16'hf0ca)
  ) __0__ (
      .I0(d),
      .I1(c),
      .I2(s0),
      .I3(s1),
      .O (tmp7)
  );
  LUT4 #(
      .INIT(16'hcaf0)
  ) __1__ (
      .I0(b),
      .I1(a),
      .I2(tmp7),
      .I3(s1),
      .O (y)
  );

  // CHECK:   INV __13__ (
  // CHECK:     .I(s0),
  // CHECK:     .O(__0__)
  // CHECK:   );
  // CHECK:   AND __14__ (
  // CHECK:     .A(b),
  // CHECK:     .B(__0__),
  // CHECK:     .Y(__1__)
  // CHECK:   );
  // CHECK:   AND __15__ (
  // CHECK:     .A(a),
  // CHECK:     .B(s0),
  // CHECK:     .Y(__2__)
  // CHECK:   );
  // CHECK:   NOR __16__ (
  // CHECK:     .A(__2__),
  // CHECK:     .B(__1__),
  // CHECK:     .Y(__3__)
  // CHECK:   );
  // CHECK:   INV __17__ (
  // CHECK:     .I(__3__),
  // CHECK:     .O(__4__)
  // CHECK:   );
  // CHECK:   AND __18__ (
  // CHECK:     .A(s1),
  // CHECK:     .B(__4__),
  // CHECK:     .Y(__5__)
  // CHECK:   );
  // CHECK:   INV __19__ (
  // CHECK:     .I(d),
  // CHECK:     .O(__6__)
  // CHECK:   );
  // CHECK:   NOR __20__ (
  // CHECK:     .A(s0),
  // CHECK:     .B(__6__),
  // CHECK:     .Y(__7__)
  // CHECK:   );
  // CHECK:   AND __21__ (
  // CHECK:     .A(c),
  // CHECK:     .B(s0),
  // CHECK:     .Y(__8__)
  // CHECK:   );
  // CHECK:   NOR __22__ (
  // CHECK:     .A(__8__),
  // CHECK:     .B(__7__),
  // CHECK:     .Y(__9__)
  // CHECK:   );
  // CHECK:   NOR __23__ (
  // CHECK:     .A(s1),
  // CHECK:     .B(__9__),
  // CHECK:     .Y(__10__)
  // CHECK:   );
  // CHECK:   NOR __24__ (
  // CHECK:     .A(__10__),
  // CHECK:     .B(__5__),
  // CHECK:     .Y(__11__)
  // CHECK:   );
  // CHECK:   INV __25__ (
  // CHECK:     .I(__11__),
  // CHECK:     .O(__12__)
  // CHECK:   );
  // CHECK:   assign y = __12__;

endmodule
