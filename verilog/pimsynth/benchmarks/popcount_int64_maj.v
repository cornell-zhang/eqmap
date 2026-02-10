// 64-bit Integer Popcount
// Dependencies: adder_1bit_half.v adder_nbit_cout.v
// deyuan, 03/30/2025

module popcount_int64_maj (
    input  [63:0] A,
    output [6:0] Y
);

    localparam WIDTH = 64;
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
    wire [2:0] sum_3bit [15:0];
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
    wire [3:0] sum_4bit [7:0];
    generate
        genvar k;
        for (k = 0; k < WIDTH; k = k + 8) begin : gen_partial_sum_4bit
            adder_nbit_cout #(
                .WIDTH(3),
                .IMPL_TYPE(IMPL_TYPE)
            ) u_adder_nbit_cout_4bit (
                .A(sum_3bit[k/4]),
                .B(sum_3bit[k/4+1]),
                .Sum(sum_4bit[k/8][2:0]),
                .Cout(sum_4bit[k/8][3])
            );
        end
    endgenerate

    // reduce to 5-bit sum
    wire [4:0] sum_5bit [3:0];
    generate
        genvar l;
        for (l = 0; l < WIDTH; l = l + 16) begin : gen_partial_sum_5bit
            adder_nbit_cout #(
                .WIDTH(4),
                .IMPL_TYPE(IMPL_TYPE)
            ) u_adder_nbit_cout_5bit (
                .A(sum_4bit[l/8]),
                .B(sum_4bit[l/8+1]),
                .Sum(sum_5bit[l/16][3:0]),
                .Cout(sum_5bit[l/16][4])
            );
        end
    endgenerate

    // reduce to 6-bit sum
    wire [5:0] sum_6bit [1:0];
    generate
        genvar m;
        for (m = 0; m < WIDTH; m = m + 32) begin : gen_partial_sum_6bit
            adder_nbit_cout #(
                .WIDTH(5),
                .IMPL_TYPE(IMPL_TYPE)
            ) u_adder_nbit_cout_6bit (
                .A(sum_5bit[m/16]),
                .B(sum_5bit[m/16+1]),
                .Sum(sum_6bit[m/32][4:0]),
                .Cout(sum_6bit[m/32][5])
            );
        end
    endgenerate

    // reduce to 7-bit sum
    generate
        genvar n;
        for (n = 0; n < WIDTH - 1; n = n + 64) begin : gen_partial_sum_7bit
            adder_nbit_cout #(
                .WIDTH(6),
                .IMPL_TYPE(IMPL_TYPE)
            ) u_adder_nbit_cout_7bit (
                .A(sum_6bit[n/32]),
                .B(sum_6bit[n/32+1]),
                .Sum(Y[n+5:n]),
                .Cout(Y[n+6])
            );
        end
    endgenerate

endmodule
