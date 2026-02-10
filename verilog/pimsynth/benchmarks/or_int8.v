// 8-bit Integer Or
// Dependencies: or_nbit.v
// deyuan, 03/28/2025

module or_int8 #(
    parameter WIDTH = 8
) (
    input [WIDTH-1:0] A,
    input [WIDTH-1:0] B,
    output [WIDTH-1:0] Y
);

    or_nbit #(
        .WIDTH(WIDTH)
    ) u_or_nbit (
        .A(A),
        .B(B),
        .Y(Y)
    );

endmodule
