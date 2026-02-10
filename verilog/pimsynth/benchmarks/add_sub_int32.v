// 32-bit Integer Addition and Subtraction
// Dependencies: adder_1bit.v adder_nbit.v subtractor_1bit.v subtractor_nbit.v
// deyuan, 03/29/2025

module add_sub_int32 #(
    parameter WIDTH = 32,
    parameter IMPL_TYPE = 0
)(
    input [WIDTH-1:0] A,
    input [WIDTH-1:0] B,
    input [WIDTH-1:0] C,
    output [WIDTH-1:0] Y
);

    wire [WIDTH-1:0] tmp;

    adder_nbit #(
        .WIDTH(WIDTH),
        .IMPL_TYPE(IMPL_TYPE)
    ) u_adder_nbit (
        .A(A),
        .B(B),
        .Sum(tmp)
    );

    subtractor_nbit #(
        .WIDTH(WIDTH),
        .IMPL_TYPE(IMPL_TYPE)
    ) u_subtractor_nbit (
        .A(tmp),
        .B(C),
        .Sub(Y)
    );

endmodule
