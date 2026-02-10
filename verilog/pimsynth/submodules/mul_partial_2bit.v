// 2-bit Partial Multiplier (only P[0] and P[1])
// Dependencies: adder_1bit.v adder_nbit.v
// hosein, 10/06/2025

module mul_partial_2bit (
    input  [1:0] A,
    input  [1:0] B,
    output [1:0] P
);

    wire p0, p1, p2;
    wire sum1, carry1;

    assign p0 = A[0] & B[0];     // P[0]
    assign p1 = A[0] & B[1];
    assign p2 = A[1] & B[0];

    adder_1bit_half ha1 (
        .A(p1),
        .B(p2),
        .Sum(sum1),
        .Cout(carry1) // Unused, as we only compute P[1:0]
    );

    assign P[0] = p0;
    assign P[1] = sum1;

endmodule

