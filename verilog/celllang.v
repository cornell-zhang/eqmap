// Copyright 2025 The EqMap Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
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
      AND _TECHMAP_REPLACE_SB (
          .A(S),
          .B(B),
          .Y(_TECHMAP_REPLACE_sb)
      );
      INV _TECHMAP_REPLACE_SI (
          .A(S),
          .Y(_TECHMAP_REPLACE_si)
      );
      AND _TECHMAP_REPLACE_SA (
          .A(_TECHMAP_REPLACE_si),
          .B(A),
          .Y(_TECHMAP_REPLACE_sa)
      );
      OR _TECHMAP_REPLACE_ (
          .A(_TECHMAP_REPLACE_sb),
          .B(_TECHMAP_REPLACE_sa),
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
      INV _TECHMAP_REPLACE_AI (
          .A(A),
          .Y(_TECHMAP_REPLACE_ai)
      );
      INV _TECHMAP_REPLACE_BI (
          .A(B),
          .Y(_TECHMAP_REPLACE_bi)
      );
      AND _TECHMAP_REPLACE_C (
          .A(A),
          .B(_TECHMAP_REPLACE_bi),
          .Y(_TECHMAP_REPLACE_c)
      );
      AND _TECHMAP_REPLACE_D (
          .A(B),
          .B(_TECHMAP_REPLACE_ai),
          .Y(_TECHMAP_REPLACE_d)
      );
      OR _TECHMAP_REPLACE_ (
          .A(_TECHMAP_REPLACE_c),
          .B(_TECHMAP_REPLACE_d),
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
