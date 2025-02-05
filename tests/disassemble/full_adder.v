
// RUN: fam %s --disassemble NOR2,INV,AND2 -s 80000 -n 40 2>>/dev/null | FileCheck %s

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

  assign s = a ^ b ^ cin;
  assign cout = (a & b) | (cin & ~a & b) | (cin & a & ~b);
  //   assign cout = (a & b) | (cin * (a ^ b));
  //   assign cout = (a & b) ^ (cin * (a ^ b));



endmodule
