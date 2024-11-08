// RUN: cargo run --release --bin parse-verilog -- %s  | FileCheck %s

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

  // CHECK: (REG d)
endmodule
