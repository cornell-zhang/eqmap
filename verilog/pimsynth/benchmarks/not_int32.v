// 32-bit Integer Not
// Dependencies: not_nbit.v
// deyuan, 03/28/2025

module not_int32 #(
    parameter WIDTH = 32
) (
    input [WIDTH-1:0] A,
    output [WIDTH-1:0] Y
);

    not_nbit #(
        .WIDTH(WIDTH)
    ) u_not_nbit (
        .A(A),
        .Y(Y)
    );

endmodule
