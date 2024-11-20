
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
      XOR2 #(
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
      XOR2 _TECHMAP_REPLACE_ (
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
      AND2 #(
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
      AND2 _TECHMAP_REPLACE_ (
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
      NOR2 _TECHMAP_REPLACE_INV (
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
      XNOR2 #(
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
      XOR2 _TECHMAP_REPLACE_INV (
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
