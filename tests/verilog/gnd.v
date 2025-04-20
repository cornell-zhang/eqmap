// RUN: cargo run --release --bin parse-verilog -- %s  | FileCheck %s

module gnd (
    d,
    y0,
    y1,
    y2
);
  input d;
  wire d;
  wire g;
  wire v;
  wire dc;
  output y0;
  wire y0;
  output y1;
  wire y1;
  output y2;
  wire y2;
  assign dc = 1'hx;
  GND GND (.G(g));
  VCC VCC (.P(v));
  AND AND_G (
      .A(d),
      .B(g),
      .Y(y0)
  );
  AND AND_V (
      .A(d),
      .B(v),
      .Y(y1)
  );
  AND AND_DC (
      .A(d),
      .B(dc),
      .Y(y2)
  );

  // CHECK: (BUS (AND d false) (AND d true) (AND d x))
endmodule
