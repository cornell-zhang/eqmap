// RUN: epak %s --disassemble NOR2,INV,AND2 -s 140000 2>>/dev/null | FileCheck %s

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

  // CHECK: NOT #(
  // CHECK: ) __10__ (
  // CHECK:     .A(s0),
  // CHECK:     .Y(__0__)
  // CHECK: );
  // CHECK: AND2 #(
  // CHECK: ) __11__ (
  // CHECK:     .A(b),
  // CHECK:     .B(__0__),
  // CHECK:     .Y(__1__)
  // CHECK: );
  // CHECK: AND2 #(
  // CHECK: ) __12__ (
  // CHECK:     .A(s0),
  // CHECK:     .B(a),
  // CHECK:     .Y(__2__)
  // CHECK: );
  // CHECK: NOR2 #(
  // CHECK: ) __13__ (
  // CHECK:     .A(__2__),
  // CHECK:     .B(__1__),
  // CHECK:     .Y(__3__)
  // CHECK: );
  // CHECK: AND2 #(
  // CHECK: ) __14__ (
  // CHECK:     .A(s1),
  // CHECK:     .B(__3__),
  // CHECK:     .Y(__4__)
  // CHECK: );
  // CHECK: NOR2 #(
  // CHECK: ) __15__ (
  // CHECK:     .A(s0),
  // CHECK:     .B(d),
  // CHECK:     .Y(__5__)
  // CHECK: );
  // CHECK: NOT #(
  // CHECK: ) __16__ (
  // CHECK:     .A(c),
  // CHECK:     .Y(__6__)
  // CHECK: );
  // CHECK: AND2 #(
  // CHECK: ) __17__ (
  // CHECK:     .A(s0),
  // CHECK:     .B(__6__),
  // CHECK:     .Y(__7__)
  // CHECK: );
  // CHECK: NOR2 #(
  // CHECK: ) __18__ (
  // CHECK:     .A(__7__),
  // CHECK:     .B(__5__),
  // CHECK:     .Y(__8__)
  // CHECK: );
  // CHECK: NOR2 #(
  // CHECK: ) __19__ (
  // CHECK:     .A(s1),
  // CHECK:     .B(__8__),
  // CHECK:     .Y(__9__)
  // CHECK: );
  // CHECK: NOR2 #(
  // CHECK: ) __20__ (
  // CHECK:     .A(__9__),
  // CHECK:     .B(__4__),
  // CHECK:     .Y(y)
  // CHECK: );

endmodule
