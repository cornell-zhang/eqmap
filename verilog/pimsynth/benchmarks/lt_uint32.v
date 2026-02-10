// 32-bit Unsigned Integer Less Than
// Dependencies: lt_uint_nbit.v subtractor_1bit_cmp.v
// deyuan, 03/30/2025

module lt_uint32 #(
    parameter WIDTH = 32,
    parameter IMPL_TYPE = 0
)(
    input [WIDTH-1:0] A,
    input [WIDTH-1:0] B,
    output Y
);

    lt_uint_nbit #(
        .WIDTH(WIDTH),
        .IMPL_TYPE(IMPL_TYPE)
    ) u_lt_uint_nbit (
        .A(A),
        .B(B),
        .Y(Y)
    );

endmodule
