// 1-bit Full Subtractor Submodule
// Dependencies: None
// deyuan, 03/28/2025

module subtractor_1bit #(
    parameter IMPL_TYPE = 0
)(
    input A,
    input B,
    input Bin,
    output Sub,
    output Bout
);

    generate
    if (IMPL_TYPE == 0) begin : impl_xor

        wire tmp;
        // XOR + MUX
        assign tmp = A ^ Bin;
        assign Bout = tmp ? Bin : B;
        assign Sub = tmp ^ B;

    end else if (IMPL_TYPE == 1) begin : impl_maj

        wire m1, m2, m3, nA, nm1;
        // MAJ + NOT
        maj3 inst1 (.A(A), .B(B), .C(Bin), .Y(m1));
        assign nA = ~A;
        maj3 inst2 (.A(nA), .B(B), .C(Bin), .Y(m2));
        assign nm1 = ~m1;
        maj3 inst3 (.A(A), .B(m2), .C(nm1), .Y(m3));
        assign Bout = m2;
        assign Sub = m3;

    end else begin : impl_unsupported
        initial begin
            $display("Unsupported implementation for subtractor_1bit: %d", IMPL_TYPE);
            $finish;
        end
    end
    endgenerate

endmodule
