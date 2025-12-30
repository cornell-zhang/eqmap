// 4-bit Unsigned Integer Multiplication
// Dependencies: adder_1bit_half.v adder_1bit.v adder_nbit.v mul_full_2bit.v mul_partial_2bit.v
// hosein, 06/10/2025

module mul_partial_4bit(
    input  [3:0] A,  // Multiplicand
    input  [3:0] B,  // Multiplier
    output [3:0] P   // Lower 4 bits of the approximate product
);

    wire [1:0] A_lo = A[1:0];
    wire [1:0] A_hi = A[3:2];
    wire [1:0] B_lo = B[1:0];
    wire [1:0] B_hi = B[3:2];

    wire [3:0] M0;
    wire [1:0] M1, M2;
    wire [1:0] M1_plus_M2;
    wire [1:0] sum_ms_bits;

    mul_full_2bit u0 (.A(A_lo), .B(B_lo), .P(M0));
    mul_partial_2bit u1 (.A(A_hi), .B(B_lo), .P(M1));
    mul_partial_2bit u2 (.A(A_lo), .B(B_hi), .P(M2));

    // Add M1 + M2 using adder_nbit 
    adder_nbit #(2) u_adder_mid (
        .A(M1), 
        .B(M2), 
        .Sum(M1_plus_M2)
    );


    // Final sum using adder_nbit
    adder_nbit #(2) u_adder_final (
        .A(M1_plus_M2),
        .B(M0[3:2]),
        .Sum(sum_ms_bits)
    );

    assign P = {sum_ms_bits, M0[1:0]};

endmodule

