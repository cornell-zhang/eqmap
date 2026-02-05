// 1-bit Half Adder Submodule
// Dependencies: maj3.v
// deyuan, 03/29/2025

module adder_1bit_half #(
    parameter IMPL_TYPE = 0
)(
    input A,
    input B,
    output Sum,
    output Cout
);

    generate
    if (IMPL_TYPE == 0) begin : impl_xor

        assign Sum = A ^ B;
        assign Cout = A & B;

    end else if (IMPL_TYPE == 1) begin : impl_maj

        wire m1, m2, m3, nm1;
        // MAJ + NOT
        maj3 inst1 (.A(A), .B(B), .C(1'b0), .Y(m1));
        assign nm1 = ~m1;
        maj3 inst2 (.A(A), .B(B), .C(1'b1), .Y(m2));
        maj3 inst3 (.A(m2), .B(nm1), .C(1'b0), .Y(m3));
        assign Cout = m1;
        assign Sum = m3;

    end else begin : impl_unsupported
        initial begin
            $display("Unsupported implementation for adder_1bit_half: %d", IMPL_TYPE);
            $finish;
        end
    end
    endgenerate

endmodule
