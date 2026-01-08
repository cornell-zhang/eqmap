// 1-bit Subtractor Submodule for Comparison
// Dependencies: None
// deyuan, 03/30/2025

module subtractor_1bit_cmp #(
    parameter IMPL_TYPE = 0
)(
    input A,
    input B,
    input Bin,
    output Bout
);

    generate
    if (IMPL_TYPE == 0) begin : impl_xor

        wire tmp;
        // XOR + MUX
        assign tmp = A ^ Bin;
        assign Bout = tmp ? Bin : B;

    end else if (IMPL_TYPE == 1) begin : impl_maj

        wire m1, nA;
        // MAJ + NOT
        assign nA = ~A;
        maj3 inst2 (.A(nA), .B(B), .C(Bin), .Y(m1));
        assign Bout = m1;

    end else begin : impl_unsupported
        initial begin
            $display("Unsupported implementation for subtractor_1bit_cmp: %d", IMPL_TYPE);
            $finish;
        end
    end
    endgenerate

endmodule
