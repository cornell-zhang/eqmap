// 8-bit Integer Popcount
// Dependencies: adder_1bit_half.v adder_nbit_cout.v
// deyuan, 03/30/2025

module popcount_int8_maj (
    input  [7:0] A,
    output [3:0] Y
);

    localparam WIDTH = 8;
    // TODO: Yosys errors out if running hierarchy with -chparam IMPL_TYPE 1
    localparam IMPL_TYPE = 1;

    // 2-bit partial sum
    wire [WIDTH-1:0] sum_2bit;
    generate
        genvar i;
        for (i = 0; i < WIDTH; i = i + 2) begin : gen_partial_sum_2bit
            adder_1bit_half #(
                .IMPL_TYPE(IMPL_TYPE)
            ) u_adder_1bit_half (
                .A(A[i]),
                .B(A[i+1]),
                .Sum(sum_2bit[i]),
                .Cout(sum_2bit[i+1])
            );
        end
    endgenerate

    // reduce to 3-bit sum
    wire [2:0] sum_3bit [1:0];
    generate
        genvar j;
        for (j = 0; j < WIDTH; j = j + 4) begin : gen_partial_sum_3bit
            adder_nbit_cout #(
                .WIDTH(2),
                .IMPL_TYPE(IMPL_TYPE)
            ) u_adder_nbit_cout_3bit (
                .A(sum_2bit[j+1:j]),
                .B(sum_2bit[j+3:j+2]),
                .Sum(sum_3bit[j/4][1:0]),
                .Cout(sum_3bit[j/4][2])
            );
        end
    endgenerate

    // reduce to 4-bit sum
    generate
        genvar k;
        for (k = 0; k < WIDTH; k = k + 8) begin : gen_partial_sum_4bit
            adder_nbit_cout #(
                .WIDTH(3),
                .IMPL_TYPE(IMPL_TYPE)
            ) u_adder_nbit_cout_4bit (
                .A(sum_3bit[k/4]),
                .B(sum_3bit[k/4+1]),
                .Sum(Y[k+2:k]),
                .Cout(Y[k+3])
            );
        end
    endgenerate

endmodule
