
// RUN: epak %s --disassemble INV,AND2 -s 80000 -n 40 -t 2 2>>/dev/null | FileCheck %s

module full_adder (
    a,
    b,
    cin,
    s,
    cout
);

  input a;
  wire a;
  input b;
  wire b;
  input cin;
  wire cin;
  output s;
  wire s;
  output cout;
  wire cout;

  AND2 _05_ (
      .A(a),
      .B(b),
      .Y(_00_)
  );
  AND2 _06_ (
      .A(cin),
      .B(_04_),
      .Y(_01_)
  );
  NOT _07_ (
      .A(_02_),
      .Y(cout)
  );
  NOR2 _08_ (
      .A(_00_),
      .B(_01_),
      .Y(_02_)
  );
  XOR2 _09_ (
      .A(a),
      .B(b),
      .Y(_03_)
  );
  XOR2 _10_ (
      .A(_03_),
      .B(cin),
      .Y(s)
  );
  XOR2 _11_ (
      .A(a),
      .B(b),
      .Y(_04_)
  );

  // CHECK: NOT #(
  // CHECK: ) __0__ (
  // CHECK:     .A(b),
  // CHECK:     .Y(tmp3)
  // CHECK: );
  // CHECK: NOT #(
  // CHECK: ) __1__ (
  // CHECK:     .A(a),
  // CHECK:     .Y(tmp5)
  // CHECK: );
  // CHECK: AND2 #(
  // CHECK: ) __2__ (
  // CHECK:     .A(tmp5),
  // CHECK:     .B(tmp3),
  // CHECK:     .Y(tmp6)
  // CHECK: );
  // CHECK: NOT #(
  // CHECK: ) __3__ (
  // CHECK:     .A(tmp6),
  // CHECK:     .Y(tmp7)
  // CHECK: );
  // CHECK: AND2 #(
  // CHECK: ) __4__ (
  // CHECK:     .A(a),
  // CHECK:     .B(b),
  // CHECK:     .Y(tmp8)
  // CHECK: );
  // CHECK: NOT #(
  // CHECK: ) __5__ (
  // CHECK:     .A(tmp8),
  // CHECK:     .Y(tmp9)
  // CHECK: );
  // CHECK: NOT #(
  // CHECK: ) __6__ (
  // CHECK:     .A(cin),
  // CHECK:     .Y(tmp11)
  // CHECK: );
  // CHECK: AND2 #(
  // CHECK: ) __7__ (
  // CHECK:     .A(tmp11),
  // CHECK:     .B(tmp9),
  // CHECK:     .Y(tmp12)
  // CHECK: );
  // CHECK: NOT #(
  // CHECK: ) __8__ (
  // CHECK:     .A(tmp12),
  // CHECK:     .Y(tmp13)
  // CHECK: );
  // CHECK: AND2 #(
  // CHECK: ) __9__ (
  // CHECK:     .A(tmp13),
  // CHECK:     .B(tmp7),
  // CHECK:     .Y(cout)
  // CHECK: );
  // CHECK: AND2 #(
  // CHECK: ) __10__ (
  // CHECK:     .A(a),
  // CHECK:     .B(tmp3),
  // CHECK:     .Y(tmp14)
  // CHECK: );
  // CHECK: NOT #(
  // CHECK: ) __11__ (
  // CHECK:     .A(tmp14),
  // CHECK:     .Y(tmp15)
  // CHECK: );
  // CHECK: AND2 #(
  // CHECK: ) __12__ (
  // CHECK:     .A(b),
  // CHECK:     .B(tmp5),
  // CHECK:     .Y(tmp16)
  // CHECK: );
  // CHECK: NOT #(
  // CHECK: ) __13__ (
  // CHECK:     .A(tmp16),
  // CHECK:     .Y(tmp17)
  // CHECK: );
  // CHECK: AND2 #(
  // CHECK: ) __14__ (
  // CHECK:     .A(tmp17),
  // CHECK:     .B(tmp15),
  // CHECK:     .Y(tmp18)
  // CHECK: );
  // CHECK: AND2 #(
  // CHECK: ) __15__ (
  // CHECK:     .A(tmp11),
  // CHECK:     .B(tmp18),
  // CHECK:     .Y(tmp19)
  // CHECK: );
  // CHECK: NOT #(
  // CHECK: ) __16__ (
  // CHECK:     .A(tmp19),
  // CHECK:     .Y(tmp20)
  // CHECK: );
  // CHECK: AND2 #(
  // CHECK: ) __17__ (
  // CHECK:     .A(cin),
  // CHECK:     .B(tmp9),
  // CHECK:     .Y(tmp21)
  // CHECK: );
  // CHECK: AND2 #(
  // CHECK: ) __18__ (
  // CHECK:     .A(tmp7),
  // CHECK:     .B(tmp21),
  // CHECK:     .Y(tmp22)
  // CHECK: );
  // CHECK: NOT #(
  // CHECK: ) __19__ (
  // CHECK:     .A(tmp22),
  // CHECK:     .Y(tmp23)
  // CHECK: );
  // CHECK: AND2 #(
  // CHECK: ) __20__ (
  // CHECK:     .A(tmp23),
  // CHECK:     .B(tmp20),
  // CHECK:     .Y(s)
  // CHECK: );

endmodule
