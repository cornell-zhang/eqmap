// RUN: eqmap_asic %s -k 4 -n 6 2>>/dev/null | FileCheck %s

module cla (
    a0,
    b0,
    a1,
    b1,
    c0,
    c2
);
  wire _00_;
  wire _01_;
  wire _02_;
  wire _03_;
  input a0;
  wire a0;
  input a1;
  wire a1;
  input b0;
  wire b0;
  input b1;
  wire b1;
  input c0;
  wire c0;
  output c2;
  wire c2;
  wire g0;
  wire g1;
  wire p0;
  wire p1;
  AND _04_ (
      .A(a0),
      .B(b0),
      .Y(g0)
  );
  AND _05_ (
      .A(a1),
      .B(b1),
      .Y(g1)
  );
  AND _06_ (
      .A(g0),
      .B(p1),
      .Y(_00_)
  );
  AND _07_ (
      .A(c0),
      .B(p0),
      .Y(_01_)
  );
  AND _08_ (
      .A(_01_),
      .B(p1),
      .Y(_02_)
  );
  OR _09_ (
      .A(a0),
      .B(b0),
      .Y(p0)
  );
  OR _10_ (
      .A(a1),
      .B(b1),
      .Y(p1)
  );
  OR _11_ (
      .A(g1),
      .B(_00_),
      .Y(_03_)
  );
  OR _12_ (
      .A(_03_),
      .B(_02_),
      .Y(c2)
  );

  // CHECK: AOI22_X1 #(
  // CHECK: ) __4__ (
  // CHECK:     .A1(a1),
  // CHECK:     .A2(b1),
  // CHECK:     .B1(a0),
  // CHECK:     .B2(b0),
  // CHECK:     .ZN(__0__)
  // CHECK: );
  // CHECK: OAI21_X1 #(
  // CHECK: ) __5__ (
  // CHECK:     .A(c0),
  // CHECK:     .B1(a0),
  // CHECK:     .B2(b0),
  // CHECK:     .ZN(__1__)
  // CHECK: );
  // CHECK: AND2_X1 #(
  // CHECK: ) __6__ (
  // CHECK:     .A1(__1__),
  // CHECK:     .A2(__0__),
  // CHECK:     .ZN(__2__)
  // CHECK: );
  // CHECK: NOR2_X1 #(
  // CHECK: ) __7__ (
  // CHECK:     .A1(a1),
  // CHECK:     .A2(b1),
  // CHECK:     .ZN(__3__)
  // CHECK: );
  // CHECK: NOR2_X1 #(
  // CHECK: ) __8__ (
  // CHECK:     .A1(__3__),
  // CHECK:     .A2(__2__),
  // CHECK:     .ZN(c2)
  // CHECK: );

endmodule
