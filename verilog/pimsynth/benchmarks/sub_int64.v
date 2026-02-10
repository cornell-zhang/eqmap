// 64-bit Integer Subtraction
// Dependencies: subtractor_1bit.v subtractor_nbit.v
// deyuan, 03/28/2025

module sub_int64 #(
    parameter WIDTH = 64,
    parameter IMPL_TYPE = 0
)(
    input [WIDTH-1:0] A,
    input [WIDTH-1:0] B,
    output [WIDTH-1:0] Sub
);

    subtractor_nbit #(
        .WIDTH(WIDTH),
        .IMPL_TYPE(IMPL_TYPE)
    ) u_subtractor_nbit (
        .A(A),
        .B(B),
        .Sub(Sub)
    );

endmodule
