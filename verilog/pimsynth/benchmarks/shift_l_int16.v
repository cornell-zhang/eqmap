// 16-bit Left Shift
// Dependencies: shift_l_nbit.v
// deyuan, 05/26/2025

module shift_l_int16 #(
    parameter WIDTH = 16
)(
    input [WIDTH-1:0] A,
    input [WIDTH-1:0] B,
    output [WIDTH-1:0] Y
);

    localparam SHIFT_WIDTH = 4;

    shift_l_nbit #(
        .WIDTH(WIDTH),
        .SHIFT_WIDTH(SHIFT_WIDTH)
    ) u_shift_l_nbit (
        .A(A),
        .B(B[SHIFT_WIDTH-1:0]),
        .Y(Y)
    );

endmodule
