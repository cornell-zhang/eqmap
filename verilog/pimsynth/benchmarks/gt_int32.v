// 32-bit Signed Integer Greater Than
// Dependencies: gt_int_nbit.v subtractor_1bit_cmp.v
// deyuan, 03/30/2025

module gt_int32 #(
    parameter WIDTH = 32,
    parameter IMPL_TYPE = 0
)(
    input [WIDTH-1:0] A,
    input [WIDTH-1:0] B,
    output Y
);

    gt_int_nbit #(
        .WIDTH(WIDTH),
        .IMPL_TYPE(IMPL_TYPE)
    ) u_gt_int_nbit (
        .A(A),
        .B(B),
        .Y(Y)
    );

endmodule
