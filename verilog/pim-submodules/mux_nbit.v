// n-bit Mux Submodule
// Dependencies: None
// joonho, 01/06/2026

module mux_nbit #(
    parameter WIDTH = 32
) (
    input [WIDTH-1:0] A,
    input [WIDTH-1:0] B,
    input sel,
    output [WIDTH-1:0] Y
);
    genvar i;
    generate
        for (i = 0; i < WIDTH; i = i + 1) begin : select_signal
            assign Y[i] = (sel & A[i]) | (~sel & B[i]);
        end
    endgenerate

endmodule
