// These are the cell mappings to convert arbitrary verilog into something parseable by EqMap
module \$mux (
    A,
    B,
    S,
    Y
);

  parameter WIDTH = 0;

  input [WIDTH-1:0] A, B;
  input S;
  output [WIDTH-1:0] Y;

  generate
    if (WIDTH == 1'd1) begin : BLOCK1
      // assign Y = S ? B : A;
      MUX _TECHMAP_REPLACE_ (
          .A(B),
          .B(A),
          .S(S),
          .Y(Y)
      );
    end else begin : BLOCK2
      // assign Y = S ? B : A;
      MUX #(
          .WIDTH(WIDTH)
      ) _TECHMAP_REPLACE_ (
          .A(B),
          .B(A),
          .S(S),
          .Y(Y)
      );
    end
  endgenerate

endmodule

module \$xor (
    A,
    B,
    Y
);

  parameter A_SIGNED = 0;
  parameter B_SIGNED = 0;
  parameter A_WIDTH = 0;
  parameter B_WIDTH = 0;
  parameter Y_WIDTH = 0;

  input [A_WIDTH-1:0] A;
  input [B_WIDTH-1:0] B;
  output [Y_WIDTH-1:0] Y;

  generate
    if (A_SIGNED || B_SIGNED || A_WIDTH > 1 || B_WIDTH > 1 || Y_WIDTH > 1) begin : BLOCK1
      XOR #(
          .A_SIGNED(A_SIGNED),
          .B_SIGNED(B_SIGNED),
          .A_WIDTH (A_WIDTH),
          .B_WIDTH (B_WIDTH),
          .Y_WIDTH (Y_WIDTH)
      ) _TECHMAP_REPLACE_ (
          .A(A),
          .B(B),
          .Y(Y)
      );
    end else begin : BLOCK2
      XOR _TECHMAP_REPLACE_ (
          .A(A),
          .B(B),
          .Y(Y)
      );
    end
  endgenerate

endmodule

module \$and (
    A,
    B,
    Y
);

  parameter A_SIGNED = 0;
  parameter B_SIGNED = 0;
  parameter A_WIDTH = 0;
  parameter B_WIDTH = 0;
  parameter Y_WIDTH = 0;

  input [A_WIDTH-1:0] A;
  input [B_WIDTH-1:0] B;
  output [Y_WIDTH-1:0] Y;

  generate
    if (A_SIGNED || B_SIGNED || A_WIDTH > 1 || B_WIDTH > 1 || Y_WIDTH > 1) begin : BLOCK1
      AND #(
          .A_SIGNED(A_SIGNED),
          .B_SIGNED(B_SIGNED),
          .A_WIDTH (A_WIDTH),
          .B_WIDTH (B_WIDTH),
          .Y_WIDTH (Y_WIDTH)
      ) _TECHMAP_REPLACE_ (
          .A(A),
          .B(B),
          .Y(Y)
      );
    end else begin : BLOCK2
      AND _TECHMAP_REPLACE_ (
          .A(A),
          .B(B),
          .Y(Y)
      );
    end
  endgenerate

endmodule

module \$not (
    A,
    Y
);

  parameter A_SIGNED = 0;
  parameter A_WIDTH = 0;
  parameter Y_WIDTH = 0;

  input [A_WIDTH-1:0] A;
  output [Y_WIDTH-1:0] Y;

  generate
    if (A_SIGNED || A_WIDTH > 1 || Y_WIDTH > 1) begin : BLOCK1
      NOT #(
          .A_SIGNED(A_SIGNED),
          .A_WIDTH (A_WIDTH),
          .Y_WIDTH (Y_WIDTH)
      ) _TECHMAP_REPLACE_ (
          .A(A),
          .Y(Y)
      );
    end else begin : BLOCK2
      NOT _TECHMAP_REPLACE_ (
          .A(A),
          .Y(Y)
      );
    end
  endgenerate

endmodule

module \$or (
    A,
    B,
    Y
);

  parameter A_SIGNED = 0;
  parameter B_SIGNED = 0;
  parameter A_WIDTH = 0;
  parameter B_WIDTH = 0;
  parameter Y_WIDTH = 0;

  input [A_WIDTH-1:0] A;
  input [B_WIDTH-1:0] B;
  output [Y_WIDTH-1:0] Y;

  generate
    if (A_SIGNED || B_SIGNED || A_WIDTH > 1 || B_WIDTH > 1 || Y_WIDTH > 1) begin : BLOCK1
      OR2 #(
          .A_SIGNED(A_SIGNED),
          .B_SIGNED(B_SIGNED),
          .A_WIDTH (A_WIDTH),
          .B_WIDTH (B_WIDTH),
          .Y_WIDTH (Y_WIDTH)
      ) _TECHMAP_REPLACE_ (
          .A(A),
          .B(B),
          .Y(Y)
      );
    end else begin : BLOCK2
      NOR _TECHMAP_REPLACE_INV (
          .A(A),
          .B(B),
          .Y(_TECHMAP_REPLACE_.inv)
      );
      NOT _TECHMAP_REPLACE_ (
          .A(_TECHMAP_REPLACE_.inv),
          .Y(Y)
      );
    end
  endgenerate

endmodule

module \$xnor (
    A,
    B,
    Y
);

  parameter A_SIGNED = 0;
  parameter B_SIGNED = 0;
  parameter A_WIDTH = 0;
  parameter B_WIDTH = 0;
  parameter Y_WIDTH = 0;

  input [A_WIDTH-1:0] A;
  input [B_WIDTH-1:0] B;
  output [Y_WIDTH-1:0] Y;

  generate
    if (A_SIGNED || B_SIGNED || A_WIDTH > 1 || B_WIDTH > 1 || Y_WIDTH > 1) begin : BLOCK1
      XNOR #(
          .A_SIGNED(A_SIGNED),
          .B_SIGNED(B_SIGNED),
          .A_WIDTH (A_WIDTH),
          .B_WIDTH (B_WIDTH),
          .Y_WIDTH (Y_WIDTH)
      ) _TECHMAP_REPLACE_ (
          .A(A),
          .B(B),
          .Y(Y)
      );
    end else begin : BLOCK2
      XOR _TECHMAP_REPLACE_INV (
          .A(A),
          .B(B),
          .Y(_TECHMAP_REPLACE_.inv)
      );
      NOT _TECHMAP_REPLACE_ (
          .A(_TECHMAP_REPLACE_.inv),
          .Y(Y)
      );
    end
  endgenerate

endmodule

module \$sdffe (
    CLK,
    SRST,
    EN,
    D,
    Q
);

  parameter WIDTH = 0;
  parameter CLK_POLARITY = 1'b1;
  parameter EN_POLARITY = 1'b1;
  parameter SRST_POLARITY = 1'b1;
  parameter SRST_VALUE = 0;

  input CLK, SRST, EN;
  input [WIDTH-1:0] D;
  output [WIDTH-1:0] Q;

  generate
    if (!SRST_POLARITY || !CLK_POLARITY || !EN_POLARITY || WIDTH > 1) begin : BLOCK1
      FDRE #(
          .SRST_POLARITY(SRST_POLARITY),
          .CLK_POLARITY(CLK_POLARITY),
          .EN_POLARITY(EN_POLARITY),
          .WIDTH(WIDTH),
          .INIT(SRST_VALUE)
      ) _TECHMAP_REPLACE_ (
          .C (CLK),
          .CE(EN),
          .D (D),
          .Q (Q),
          .R (SRST)
      );
    end else begin : BLOCK2
      FDRE #(
          .INIT(SRST_VALUE)
      ) _TECHMAP_REPLACE_ (
          .C (CLK),
          .CE(EN),
          .D (D),
          .Q (Q),
          .R (SRST)
      );
    end
  endgenerate

endmodule

module \$dff (
    CLK,
    D,
    Q
);

  parameter WIDTH = 0;
  parameter CLK_POLARITY = 1'b1;

  input CLK;
  input [WIDTH-1:0] D;
  output [WIDTH-1:0] Q;


  generate
    if (!CLK_POLARITY || WIDTH > 1) begin : BLOCK1
      FDRE #(
          .WIDTH(WIDTH),
          .INIT (1'hx)
      ) _TECHMAP_REPLACE_ (
          .C (CLK),
          .CE(1'b1),
          .D (D),
          .Q (Q),
          .R (1'b0)
      );
    end else begin : BLOCK2
      FDRE #(
          .INIT(1'hx)
      ) _TECHMAP_REPLACE_ (
          .C (CLK),
          .CE(1'b1),
          .D (D),
          .Q (Q),
          .R (1'b0)
      );
    end
  endgenerate

endmodule
