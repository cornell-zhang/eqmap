// n-bit Or Submodule
// Dependencies: None
// deyuan, 03/28/2025

module or_nbit #(
    parameter WIDTH = 32
) (
    input [WIDTH-1:0] A,
    input [WIDTH-1:0] B,
    output [WIDTH-1:0] Y
);

    assign Y = A | B;

endmodule
