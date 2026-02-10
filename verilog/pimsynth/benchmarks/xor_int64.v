// 64-bit Integer Xor
// Dependencies: xor_nbit.v
// deyuan, 03/28/2025

module xor_int64 #(
    parameter WIDTH = 64
) (
    input [WIDTH-1:0] A,
    input [WIDTH-1:0] B,
    output [WIDTH-1:0] Y
);

    xor_nbit #(
        .WIDTH(WIDTH)
    ) u_xor_nbit (
        .A(A),
        .B(B),
        .Y(Y)
    );

endmodule
