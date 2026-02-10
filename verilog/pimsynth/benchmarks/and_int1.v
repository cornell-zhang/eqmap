// 1-bit Integer And
// Dependencies: and_nbit.v
// deyuan, 03/28/2025

module and_int1 #(
    parameter WIDTH = 1
) (
    input [WIDTH-1:0] A,
    input [WIDTH-1:0] B,
    output [WIDTH-1:0] Y
);

    and_nbit #(
        .WIDTH(WIDTH)
    ) u_and_nbit (
        .A(A),
        .B(B),
        .Y(Y)
    );

endmodule
