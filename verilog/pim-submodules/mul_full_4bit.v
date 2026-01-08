// 4-bit Unsigned Integer Multiplication
// Dependencies: adder_1bit_half.v adder_1bit.v adder_nbit.v mul_full_2bit.v 
// hosein, 06/10/2025

module mul_full_4bit(
    input  [3:0] A,
    input  [3:0] B,
    output [7:0] P
);
    localparam WIDTH = 2;

    wire [WIDTH-1:0] A_lo = A[WIDTH-1:0];
    wire [WIDTH-1:0] A_hi = A[2*WIDTH-1:WIDTH];
    wire [WIDTH-1:0] B_lo = B[WIDTH-1:0];
    wire [WIDTH-1:0] B_hi = B[2*WIDTH-1:WIDTH];

    wire [2*WIDTH-1:0] M0, M1, M2, M3;
    wire [4*WIDTH-1:0] M0_ext, M1_ext, M2_ext, M3_ext;
    wire [4*WIDTH-1:0] sum1, sum2;

    mul_full_2bit u0 (.A(A_lo), .B(B_lo), .P(M0));
    mul_full_2bit u1 (.A(A_hi), .B(B_lo), .P(M1));
    mul_full_2bit u2 (.A(A_lo), .B(B_hi), .P(M2));
    mul_full_2bit u3 (.A(A_hi), .B(B_hi), .P(M3));

    assign M0_ext = { {2*WIDTH{1'b0}}, M0 };
    assign M1_ext = { {WIDTH{1'b0}}, M1, {WIDTH{1'b0}} };
    assign M2_ext = { {WIDTH{1'b0}}, M2, {WIDTH{1'b0}} };
    assign M3_ext = { M3, {2*WIDTH{1'b0}} };

    adder_nbit #(4*WIDTH) add1 (.A(M1_ext), .B(M2_ext), .Sum(sum1));
    adder_nbit #(4*WIDTH) add2 (.A(sum1), .B(M3_ext), .Sum(sum2));
    adder_nbit #(4*WIDTH) add3 (.A(sum2), .B(M0_ext), .Sum(P));

endmodule

