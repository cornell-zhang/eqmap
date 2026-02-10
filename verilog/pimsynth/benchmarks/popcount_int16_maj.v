// 16-bit Integer Popcount
// Dependencies: adder_1bit_half.v adder_nbit_cout.v
// deyuan, 03/30/2025

module popcount_int16_maj (
    input  [15:0] A,
    output [4:0] Y
);

    localparam WIDTH = 16;
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
    wire [2:0] sum_3bit [3:0];
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
    wire [3:0] sum_4bit [1:0];
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
    generate
        genvar l;
        for (l = 0; l < WIDTH; l = l + 16) begin : gen_partial_sum_5bit
            adder_nbit_cout #(
                .WIDTH(4),
                .IMPL_TYPE(IMPL_TYPE)
            ) u_adder_nbit_cout_5bit (
                .A(sum_4bit[l/8]),
                .B(sum_4bit[l/8+1]),
                .Sum(Y[l+3:l]),
                .Cout(Y[l+4])
            );
        end
    endgenerate

endmodule
