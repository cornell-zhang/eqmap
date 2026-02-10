// n-bit NE (not-equal) Submodule
// Dependencies: None
// deyuan, 03/30/2025

module ne_nbit #(
    parameter WIDTH = 32
)(
    input [WIDTH-1:0] A,
    input [WIDTH-1:0] B,
    output wire Y
);

    wire [WIDTH:0] ne;
    assign ne[0] = 1'b0;
    genvar i;
    generate
        for (i = 0; i < WIDTH; i = i + 1) begin : eq_chain
            assign ne[i+1] = ne[i] | (A[i] ^ B[i]);
        end
    endgenerate

    assign Y = ne[WIDTH];

endmodule