// RUN: eqmap_fpga %s --verify --assert-sat -k 4 | FileCheck %s

module mux_reg (
    input  wire a,
    input  wire b,
    input  wire c,
    input  wire clk,
    input  wire d,
    input  wire s0,
    input  wire s1,
    output wire y
);
  wire tmp0;
  wire tmp1;
  wire tmp0_r;
  wire tmp1_r;
  wire s1_r;
  LUT3 #(
      .INIT(8'hCA)
  ) _0_ (
      .I0(b),
      .I1(a),
      .I2(s0),
      .O (tmp1)
  );
  LUT3 #(
      .INIT(8'hCA)
  ) _1_ (
      .I0(d),
      .I1(c),
      .I2(s0),
      .O (tmp0)
  );
  FDRE #(
      .INIT(1'bx)
  ) _pipe_tmp0_ (
      .C (clk),
      .CE(1'b1),
      .D (tmp0),
      .Q (tmp0_r),
      .R (1'b0)
  );
  FDRE #(
      .INIT(1'bx)
  ) _pipe_tmp1_ (
      .C (clk),
      .CE(1'b1),
      .D (tmp1),
      .Q (tmp1_r),
      .R (1'b0)
  );
  FDRE #(
      .INIT(1'bx)
  ) _pipe_s1_ (
      .C (clk),
      .CE(1'b1),
      .D (s1),
      .Q (s1_r),
      .R (1'b0)
  );
  LUT3 #(
      .INIT(8'hCA)
  ) _2_ (
      .I0(tmp0_r),
      .I1(tmp1_r),
      .I2(s1_r),
      .O (y)
  );
endmodule

// CHECK: module mux_reg (
// CHECK:   input wire a,
// CHECK:   input wire b,
// CHECK:   input wire c,
// CHECK:   input wire clk,
// CHECK:   input wire d,
// CHECK:   input wire s0,
// CHECK:   input wire s1,
// CHECK:   output wire y
// CHECK: );
// CHECK:   wire __2__;
// CHECK:   wire __3__;
// CHECK:   wire __4__;
// CHECK:   LUT4 #(
// CHECK:     .INIT(16'hf0ca)
// CHECK:   ) __7__ (
// CHECK:     .I3(s1),
// CHECK:     .I2(s0),
// CHECK:     .I1(c),
// CHECK:     .I0(d),
// CHECK:     .O(__2__)
// CHECK:   );
// CHECK:   LUT4 #(
// CHECK:     .INIT(16'hcaf0)
// CHECK:   ) __8__ (
// CHECK:     .I3(s1),
// CHECK:     .I2(__2__),
// CHECK:     .I1(a),
// CHECK:     .I0(b),
// CHECK:     .O(__3__)
// CHECK:   );
// CHECK:   FDRE #(
// CHECK:     .INIT(1'bx)
// CHECK:   ) __9__ (
// CHECK:     .D(__3__),
// CHECK:     .C(clk),
// CHECK:     .CE(1'b1),
// CHECK:     .R(1'b0),
// CHECK:     .Q(__4__)
// CHECK:   );
// CHECK:   assign y = __4__;
// CHECK: endmodule
