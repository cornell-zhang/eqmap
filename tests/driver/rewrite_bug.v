// RUN: eqmap_fpga %s --assert-sat --no-retime -k 4 | FileCheck %s

module rewrite_bug (
    clk,
    rst,
    a,
    y
);
  wire _0_;
  input a;
  wire a;
  input clk;
  wire clk;
  wire p;
  wire q;
  input rst;
  wire rst;
  output y;
  wire y;
  LUT2 #(
      .INIT(4'h5)
  ) _1_ (
      .I0(q),
      .I1(p),
      .O (_0_)
  );
  FDRE #(
      .INIT(1'hx)
  ) _2_ (
      .C (clk),
      .CE(1'h1),
      .D (a),
      .Q (q),
      .R (rst)
  );
  FDRE #(
      .INIT(1'hx)
  ) _3_ (
      .C (clk),
      .CE(1'h1),
      .D (q),
      .Q (p),
      .R (1'h0)
  );
  FDRE #(
      .INIT(1'hx)
  ) _4_ (
      .C (clk),
      .CE(1'h1),
      .D (_0_),
      .Q (y),
      .R (rst)
  );

  // CHECK: FDRE #(
  // CHECK:   .INIT(1'bx)
  // CHECK: ) __4__ (
  // CHECK:   .D(a),
  // CHECK:   .C(clk),
  // CHECK:   .CE(1'b1),
  // CHECK:   .R(rst),
  // CHECK:   .Q(__0__)
  // CHECK: );
  // CHECK: FDRE #(
  // CHECK:   .INIT(1'bx)
  // CHECK: ) __5__ (
  // CHECK:   .D(__3__),
  // CHECK:   .C(clk),
  // CHECK:   .CE(1'b1),
  // CHECK:   .R(rst),
  // CHECK:   .Q(__1__)
  // CHECK: );
  // CHECK: LUT1 #(
  // CHECK:   .INIT(2'b01)
  // CHECK: ) __7__ (
  // CHECK:   .I0(__0__),
  // CHECK:   .O(__3__)
  // CHECK: );
  // CHECK: assign y = __1__;

endmodule
