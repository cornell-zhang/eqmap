// 8-bit Integer EQ
// Dependencies: eq_nbit.v
// deyuan, 03/30/2025

module eq_int8 #(
    parameter WIDTH = 8
)(
    input [WIDTH-1:0] A,
    input [WIDTH-1:0] B,
    output Y
);

    eq_nbit #(
        .WIDTH(WIDTH)
    ) u_eq_nbit (
        .A(A),
        .B(B),
        .Y(Y)
    );

endmodule