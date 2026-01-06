// n-bit Integer Multiplier Submodule
// Dependencies: adder_1bit.v adder_nbit.v, mux_nbit.v
// hosein, 10/06/2025

module multiplier_nbit #(
    parameter WIDTH = 32,
    parameter IMPL_TYPE = 0
)(
    input  [WIDTH-1:0] A,  // Multiplicand
    input  [WIDTH-1:0] B,  // Multiplier
    output [WIDTH-1:0] P   // Lower WIDTH bits of the product
);

    // Create partial products for each B[i]
    wire [WIDTH-1:0] acc [0:WIDTH];
    // Assign initial value for acc[0]
    mux_nbit #(.WIDTH(WIDTH)) acc_0_sel ( .A(A), .B({WIDTH{1'b0}}), .sel(B[0]), .Y(acc[0]));

    // Generate a chain of bit-serial adders
    genvar i;
    generate
            for (i = 1; i < WIDTH; i = i + 1) begin : chain_adders
            assign acc[i][i-1:0] = acc[i-1][i-1:0];
            wire [WIDTH-i-1:0] u_a_input;
            mux_nbit #(.WIDTH(WIDTH-1)) u_mux_nbit ( .A(A[WIDTH-i-1:0]), .B({WIDTH-i{1'b0}}), .sel(B[i]), .Y(u_a_input));
            adder_nbit #(.WIDTH(WIDTH-i), .IMPL_TYPE(IMPL_TYPE)) u_adder_nbit (
                .A(u_a_input),
                .B(acc[i-1][WIDTH-1:i]),
                .Sum(acc[i][WIDTH-1:i])
            );
        end
    endgenerate

    assign P = acc[WIDTH-1];

endmodule
