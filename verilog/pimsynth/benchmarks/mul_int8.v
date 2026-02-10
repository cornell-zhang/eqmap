// 8-bit Integer Multiplication
// Dependencies: multiplier_nbit.v adder_1bit.v adder_nbit.v
// deyuan, 03/28/2025

module mul_int8 #(
    parameter WIDTH = 8,
    parameter IMPL_TYPE = 0
)(
    input  [WIDTH-1:0] A,  // Multiplicand
    input  [WIDTH-1:0] B,  // Multiplier
    output [WIDTH-1:0] P   // Lower WIDTH bits of the product
);

    // Instantiate the n-bit multiplier submodule
    multiplier_nbit #(
        .WIDTH(WIDTH),
        .IMPL_TYPE(IMPL_TYPE)
    ) u_multiplier_nbit (
        .A(A),
        .B(B),
        .P(P)
    );

endmodule
