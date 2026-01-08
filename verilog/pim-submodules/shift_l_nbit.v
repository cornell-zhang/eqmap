// n-bit Left Shift Submodule
// Dependencies: None
// deyuan, 05/26/2025

module shift_l_nbit #(
    parameter WIDTH = 32,
    parameter SHIFT_WIDTH = 5
)(
    input [WIDTH-1:0] A,
    input [SHIFT_WIDTH-1:0] B,
    output [WIDTH-1:0] Y
);

wire [WIDTH-1:0] shift [0:SHIFT_WIDTH-1];

genvar i;
generate
    for (i = 0; i < SHIFT_WIDTH; i = i + 1) begin : gen_shift
        if (i == 0) begin
            assign shift[i] = B[i] ? (A << (1 << i)) : A;
        end else begin
            assign shift[i] = B[i] ? (shift[i-1] << (1 << i)) : shift[i-1];
        end
    end
endgenerate

assign Y = shift[SHIFT_WIDTH-1];

endmodule
