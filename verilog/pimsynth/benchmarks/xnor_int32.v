// 32-bit Integer Xnor
// Dependencies: xnor_nbit.v
// deyuan, 03/28/2025

module xnor_int32 #(
    parameter WIDTH = 32
) (
    input [WIDTH-1:0] A,
    input [WIDTH-1:0] B,
    output [WIDTH-1:0] Y
);

    xnor_nbit #(
        .WIDTH(WIDTH)
    ) u_xnor_nbit (
        .A(A),
        .B(B),
        .Y(Y)
    );

endmodule
