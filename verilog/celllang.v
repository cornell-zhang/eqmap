
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
      INV #(
          .A_SIGNED(A_SIGNED),
          .A_WIDTH (A_WIDTH),
          .Y_WIDTH (Y_WIDTH)
      ) _TECHMAP_REPLACE_ (
          .A(A),
          .Y(Y)
      );
    end else begin : BLOCK2
      INV _TECHMAP_REPLACE_ (
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
      OR #(
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
      OR _TECHMAP_REPLACE_INV (
          .A(A),
          .B(B),
          .Y(Y)
      );
    end
  endgenerate

endmodule
