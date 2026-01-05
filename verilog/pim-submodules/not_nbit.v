// n-bit Not Submodule
// Dependencies: None
// deyuan, 03/28/2025

module not_nbit #(
    parameter WIDTH = 32
) (
    input [WIDTH-1:0] A,
    output [WIDTH-1:0] Y
);

    assign Y = ~A;

endmodule
