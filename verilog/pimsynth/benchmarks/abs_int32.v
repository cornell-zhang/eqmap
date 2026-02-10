// 32-bit Integer Abs
// Dependencies: abs_nbit.v adder_1bit_half.v
// deyuan, 03/29/2025

module abs_int32 #(
    parameter WIDTH = 32,
    parameter IMPL_TYPE = 0
)(
    input [WIDTH-1:0] A,
    output [WIDTH-1:0] Y
);

    abs_nbit #(
        .WIDTH(WIDTH),
        .IMPL_TYPE(IMPL_TYPE)
    ) u_abs_nbit (
        .A(A),
        .Y(Y)
    );

endmodule
