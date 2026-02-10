// 32-bit Integer NE
// Dependencies: ne_nbit.v
// deyuan, 03/30/2025

module ne_int32 #(
    parameter WIDTH = 32
)(
    input [WIDTH-1:0] A,
    input [WIDTH-1:0] B,
    output Y
);

    ne_nbit #(
        .WIDTH(WIDTH)
    ) u_ne_nbit (
        .A(A),
        .B(B),
        .Y(Y)
    );

endmodule
