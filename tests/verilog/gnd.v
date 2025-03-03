// RUN: cargo run --release --bin parse-verilog -- %s  | FileCheck %s

module gnd (
    d,
    y0,
    y1
);
  input d;
  wire d;
  wire g;
  wire v;
  output y0;
  wire y0;
  output y1;
  wire y1;
  GND GND (.G(g));
  VCC VCC (.P(v));
  AND2 AND_G (
      .A(d),
      .B(g),
      .Y(y0)
  );
  AND2 AND_V (
      .A(d),
      .B(v),
      .Y(y1)
  );

  // CHECK: (BUS (AND d false) (AND d true))
endmodule
