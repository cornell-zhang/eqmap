// n-bit Adder Submodule with Carry Out
// Dependencies: adder_1bit.v
// deyuan, 03/28/2025

module adder_nbit_cout #(
    parameter WIDTH = 32,
    parameter IMPL_TYPE = 0
)(
    input [WIDTH-1:0] A,
    input [WIDTH-1:0] B,
    output [WIDTH-1:0] Sum,
    output Cout
);

    wire [WIDTH-1:0] Carry;

    // Instantiate the first full adder with Cin = 0
    adder_1bit #(
        .IMPL_TYPE(IMPL_TYPE)
    ) u_adder_1bit_0 (
        .A(A[0]),
        .B(B[0]),
        .Cin(1'b0),    // Initialize Cin to 0
        .Sum(Sum[0]),
        .Cout(Carry[0])
    );

    // Instantiate the remaining WIDTH-1 full adders
    genvar i;
    generate
        for (i = 1; i < WIDTH; i = i + 1) begin : adder_chain
            adder_1bit #(
                .IMPL_TYPE(IMPL_TYPE)
            ) u_adder_1bit (
                .A(A[i]),
                .B(B[i]),
                .Cin(Carry[i-1]),
                .Sum(Sum[i]),
                .Cout(Carry[i])
            );
        end
    endgenerate

    assign Cout = Carry[WIDTH-1];

endmodule
