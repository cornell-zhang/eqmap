// 8-bit Signed Integer Max
// Dependencies: gt_int_nbit.v subtractor_1bit_cmp.v
// deyuan, 03/30/2025

module max_int8 #(
    parameter WIDTH = 8,
    parameter IMPL_TYPE = 0
)(
    input [WIDTH-1:0] A,
    input [WIDTH-1:0] B,
    output [WIDTH-1:0] Y,
);

    wire gt;
    gt_int_nbit #(
        .WIDTH(WIDTH),
        .IMPL_TYPE(IMPL_TYPE)
    ) u_gt_int_nbit (
        .A(A),
        .B(B),
        .Y(gt)
    );

    assign Y = gt ? A : B;

endmodule
