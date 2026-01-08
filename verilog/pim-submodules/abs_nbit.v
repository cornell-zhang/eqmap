// n-bit Abs Submodule
// Dependencies: adder_1bit_half.v
// deyuan, 03/29/2025

module abs_nbit #(
    parameter WIDTH = 32,
    parameter IMPL_TYPE = 0
)(
    input [WIDTH-1:0] A,
    output [WIDTH-1:0] Y
);

    wire [WIDTH:0] Carry;
    assign Carry[0] = A[WIDTH-1];

    // Instantiate a chain of half adders
    genvar i;
    generate
        for (i = 0; i < WIDTH; i = i + 1) begin : half_adder_chain
            adder_1bit_half #(
                .IMPL_TYPE(IMPL_TYPE)
            ) u_adder_1bit_half (
                .A(A[i] ^ A[WIDTH-1]), // perform NOT if sign bit is 1
                .B(Carry[i]),
                .Sum(Y[i]),
                .Cout(Carry[i+1])
            );
        end
    endgenerate

endmodule
