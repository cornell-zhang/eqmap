// RUN: epak %s --disassemble NOR2,INV,AND2 -s 140000 -n 40 2>>/dev/null | FileCheck %s

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
  // CHECK: ) __0__ (
  // CHECK:     .A(s0),
  // CHECK:     .Y(tmp2)
  // CHECK: );
  // CHECK: AND2 #(
  // CHECK: ) __1__ (
  // CHECK:     .A(b),
  // CHECK:     .B(tmp2),
  // CHECK:     .Y(tmp4)
  // CHECK: );
  // CHECK: AND2 #(
  // CHECK: ) __2__ (
  // CHECK:     .A(s0),
  // CHECK:     .B(a),
  // CHECK:     .Y(tmp6)
  // CHECK: );
  // CHECK: NOR2 #(
  // CHECK: ) __3__ (
  // CHECK:     .A(tmp6),
  // CHECK:     .B(tmp4),
  // CHECK:     .Y(tmp7)
  // CHECK: );
  // CHECK: AND2 #(
  // CHECK: ) __4__ (
  // CHECK:     .A(s1),
  // CHECK:     .B(tmp7),
  // CHECK:     .Y(tmp9)
  // CHECK: );
  // CHECK: NOR2 #(
  // CHECK: ) __5__ (
  // CHECK:     .A(s0),
  // CHECK:     .B(d),
  // CHECK:     .Y(tmp11)
  // CHECK: );
  // CHECK: NOT #(
  // CHECK: ) __6__ (
  // CHECK:     .A(c),
  // CHECK:     .Y(tmp13)
  // CHECK: );
  // CHECK: AND2 #(
  // CHECK: ) __7__ (
  // CHECK:     .A(s0),
  // CHECK:     .B(tmp13),
  // CHECK:     .Y(tmp14)
  // CHECK: );
  // CHECK: NOR2 #(
  // CHECK: ) __8__ (
  // CHECK:     .A(tmp14),
  // CHECK:     .B(tmp11),
  // CHECK:     .Y(tmp15)
  // CHECK: );
  // CHECK: NOR2 #(
  // CHECK: ) __9__ (
  // CHECK:     .A(s1),
  // CHECK:     .B(tmp15),
  // CHECK:     .Y(tmp16)
  // CHECK: );
  // CHECK: NOR2 #(
  // CHECK: ) __10__ (
  // CHECK:     .A(tmp16),
  // CHECK:     .B(tmp9),
  // CHECK:     .Y(y)
  // CHECK: );

endmodule
