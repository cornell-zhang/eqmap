// n-bit Left Shift Submodule
// Dependencies: mux_nbit.v
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
            mux_nbit #(.WIDTH(WIDTH)) u_mux_nbit_0 ( .A(A << (1 << i)), .B(A), .sel(B[i]), .Y(shift[i]));
        end else begin
            mux_nbit #(.WIDTH(WIDTH)) u_mux_nbit_1 ( .A(shift[i-1] << (1 << i)), .B(shift[i-1]), .sel(B[i]), .Y(shift[i]));
        end
    end
endgenerate

assign Y = shift[SHIFT_WIDTH-1];

endmodule
