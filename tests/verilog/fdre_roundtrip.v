// RUN: cargo run --release --bin parse-verilog -- %s --round-trip | FileCheck %s

module fdre (
    d,
    clk,
    y
);
  input d;
  wire d;
  input clk;
  wire clk;
  output y;
  wire y;
  FDRE #(
      .INIT(1'hx)
  ) _4_ (
      .C (clk),
      .CE(1'h1),
      .D (d),
      .Q (y),
      .R (1'h0)
  );

endmodule

// CHECK: module fdre (
// CHECK:     d,
// CHECK:     clk,
// CHECK:     y
// CHECK: );
// CHECK:   input d;
// CHECK:   wire d;
// CHECK:   input clk;
// CHECK:   wire clk;
// CHECK:   output y;
// CHECK:   wire y;
// CHECK:   wire _4_CE_const;
// CHECK:   wire _4_R_const;
// CHECK:   assign _4_CE_const = 1'b1;
// CHECK:   assign _4_R_const = 1'b0;
// CHECK:   FDRE #(
// CHECK:       .INIT(1'hx)
// CHECK:   ) _4_ (
// CHECK:       .C(clk),
// CHECK:       .CE(_4_CE_const),
// CHECK:       .D(d),
// CHECK:       .R(_4_R_const),
// CHECK:       .Q(y)
// CHECK:   );
// CHECK: endmodule
