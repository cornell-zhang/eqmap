// 2-bit Integer Unsigned Multiplier Submodule  
// Dependencies: adder_1bit.v adder_1bit_half.v  
// hosein, 10/06/2025

module mul_full_2bit (
    input  [1:0] A,
    input  [1:0] B,
    output [3:0] P
);

    wire p0, p1, p2, p3;         // partial products
    wire s1, c1;                 // intermediate sums and carries
    wire s2, c2;

    assign p0 = A[0] & B[0];     // LSB
    assign p1 = A[0] & B[1];
    assign p2 = A[1] & B[0];
    assign p3 = A[1] & B[1];     // MSB partial

    wire sum1, carry1;
    adder_1bit_half ha1 (
        .A(p1),
        .B(p2),
        .Sum(sum1),
        .Cout(carry1)
    );

    wire sum2, carry2;
    adder_1bit fa1 (
        .A(p3),
        .B(1'b0),
        .Cin(carry1),
        .Sum(sum2),
        .Cout(carry2)
    );

    assign P[0] = p0;
    assign P[1] = sum1;
    assign P[2] = sum2;
    assign P[3] = carry2;

endmodule

