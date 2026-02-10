// n-bit Signed Integer Greater Than Submodule
// Dependencies: gt_uint_nbit.v subtractor_1bit_cmp.v
// deyuan, 03/30/2025

module gt_int_nbit #(
    parameter WIDTH = 32,
    parameter IMPL_TYPE = 0
)(
    input [WIDTH-1:0] A,
    input [WIDTH-1:0] B,
    output Y
);

    // n-1 bit uint gt
    wire tmp1;
    gt_uint_nbit #(
        .WIDTH(WIDTH-1),
        .IMPL_TYPE(IMPL_TYPE)
    ) u_gt_uint_nbit (
        .A(A[WIDTH-2:0]),
        .B(B[WIDTH-2:0]),
        .Y(tmp1)
    );

    // handle sign bit
    wire tmp2 = A[WIDTH-1] ^ B[WIDTH-1];
    assign Y = tmp2 ? B[WIDTH-1] : tmp1;

endmodule
