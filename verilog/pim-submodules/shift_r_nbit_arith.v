// n-bit Right Shift Submodule
// Dependencies: None
// deyuan, 05/26/2025

module shift_r_nbit_arith #(
    parameter WIDTH = 32,
    parameter SHIFT_WIDTH = 5
)(
    input signed [WIDTH-1:0] A,
    input [SHIFT_WIDTH-1:0] B,
    output signed [WIDTH-1:0] Y
);

wire signed [WIDTH-1:0] shift [0:SHIFT_WIDTH-1];

genvar i;
generate
    for (i = 0; i < SHIFT_WIDTH; i = i + 1) begin : gen_shift
        if (i == 0) begin
            assign shift[i] = B[i] ? ($signed(A) >>> (1 << i)) : A;
        end else begin
            assign shift[i] = B[i] ? ($signed(shift[i-1]) >>> (1 << i)) : shift[i-1];
        end
    end
endgenerate

assign Y = shift[SHIFT_WIDTH-1];

endmodule
