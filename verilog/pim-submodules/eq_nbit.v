// n-bit EQ (equal) Submodule
// Dependencies: ne_nbit.v
// deyuan, 03/30/2025

module eq_nbit #(
    parameter WIDTH = 32
)(
    input [WIDTH-1:0] A,
    input [WIDTH-1:0] B,
    output wire Y
);

    wire ne;
    ne_nbit #(
        .WIDTH(WIDTH)
    ) u_ne_nbit (
        .A(A),
        .B(B),
        .Y(ne)
    );

    assign Y = ~ne;

endmodule