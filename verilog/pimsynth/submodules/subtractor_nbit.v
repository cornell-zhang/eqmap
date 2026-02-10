// n-bit Subtractor Submodule
// Dependencies: subtractor_1bit.v
// deyuan, 03/28/2025

module subtractor_nbit #(
    parameter WIDTH = 32,
    parameter IMPL_TYPE = 0
)(
    input [WIDTH-1:0] A,
    input [WIDTH-1:0] B,
    output [WIDTH-1:0] Sub
);

    wire [WIDTH-1:0] Borrow;

    // Instantiate the first full subtractor with Bin = 0
    subtractor_1bit #(
        .IMPL_TYPE(IMPL_TYPE)
    ) u_subtractor_1bit (
        .A(A[0]),
        .B(B[0]),
        .Bin(1'b0), // Initialize Bin to 0
        .Sub(Sub[0]),
        .Bout(Borrow[0])
    );

    // Instantiate the remaining WIDTH-1 full subtractors
    genvar i;
    generate
        for (i = 1; i < WIDTH; i = i + 1) begin : subtractor_chain
            subtractor_1bit #(
                .IMPL_TYPE(IMPL_TYPE)
            ) u_subtractor_1bit (
                .A(A[i]),
                .B(B[i]),
                .Bin(Borrow[i-1]),
                .Sub(Sub[i]),
                .Bout(Borrow[i])
            );
        end
    endgenerate

endmodule
