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

module LUT1 #(
    parameter INIT = 2'h0
) (
    input  I0,
    output O
);
  assign O = INIT[I0];
endmodule

module LUT2 #(
    parameter INIT = 4'h0
) (
    input  I0,
    input  I1,
    output O
);
  assign O = INIT[{I1, I0}];
endmodule


module LUT3 #(
    parameter INIT = 8'h00
) (
    input  I0,
    input  I1,
    input  I2,
    output O
);
  assign O = INIT[{I2, I1, I0}];
endmodule

module LUT4 #(
    parameter INIT = 16'h0000
) (
    input  I0,
    input  I1,
    input  I2,
    input  I3,
    output O
);
  assign O = INIT[{I3, I2, I1, I0}];
endmodule

module LUT5 #(
    parameter INIT = 32'h00000000
) (
    input  I0,
    input  I1,
    input  I2,
    input  I3,
    input  I4,
    output O
);
  assign O = INIT[{I4, I3, I2, I1, I0}];
endmodule

module LUT6 #(
    parameter INIT = 64'h0000000000000000
) (
    input  I0,
    input  I1,
    input  I2,
    input  I3,
    input  I4,
    input  I5,
    output O
);
  assign O = INIT[{I5, I4, I3, I2, I1, I0}];
endmodule

module AND2 (
    input  A,
    input  B,
    output Y
);
  assign Y = A & B;
endmodule

module NAND2 (
    input  A,
    input  B,
    output Y
);
  assign Y = ~(A & B);
endmodule


module NOR2 (
    input  A,
    input  B,
    output Y
);
  assign Y = ~(A | B);
endmodule

module OR2 (
    input  A,
    input  B,
    output Y
);
  assign Y = A | B;
endmodule

module XOR2 (
    input  A,
    input  B,
    output Y
);
  assign Y = A ^ B;
endmodule

module AND (
    input  A,
    input  B,
    output Y
);
  assign Y = A & B;
endmodule

module NAND (
    input  A,
    input  B,
    output Y
);
  assign Y = ~(A & B);
endmodule


module NOR (
    input  A,
    input  B,
    output Y
);
  assign Y = ~(A | B);
endmodule

module OR (
    input  A,
    input  B,
    output Y
);
  assign Y = A | B;
endmodule

module XOR (
    input  A,
    input  B,
    output Y
);
  assign Y = A ^ B;
endmodule

module NOT (
    input  A,
    output Y
);
  assign Y = ~A;
endmodule

module INV (
    input  A,
    output ZN
);
  assign ZN = ~A;
endmodule

module MUX (
    input  A,
    input  B,
    input  S,
    output Y
);
  assign Y = S ? A : B;
endmodule


module FDRE #(
    parameter INIT = 0
) (
    input  C,
    input  CE,
    input  D,
    input  R,
    output Q
);
  reg q;
  always @(posedge C) begin
    if (CE) begin
      if (R) begin
        q <= INIT;
      end else begin
        q <= D;
      end
    end
  end
endmodule
