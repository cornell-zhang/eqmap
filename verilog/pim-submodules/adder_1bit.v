// 1-bit Full Adder Submodule
// Dependencies: maj3.v
// deyuan, 03/28/2025

module adder_1bit #(
    parameter IMPL_TYPE = 0
)(
    input A,
    input B,
    input Cin,
    output Sum,
    output Cout
);

    generate
    if (IMPL_TYPE == 0) begin : impl_xor

        wire tmp;
        // XOR + MUX
        assign tmp = A ^ Cin;
        assign Cout = tmp ? B : Cin;
        assign Sum = tmp ^ B;

    end else if (IMPL_TYPE == 1) begin : impl_maj

        wire m1, m2, m3, nCin, nm1;
        // MAJ + NOT
        maj3 inst1 (.A(A), .B(B), .C(Cin), .Y(m1));
        assign nm1 = ~m1;
        assign nCin = ~Cin;
        maj3 inst2 (.A(A), .B(B), .C(nCin), .Y(m2));
        maj3 inst3 (.A(m2), .B(nm1), .C(Cin), .Y(m3));
        assign Cout = m1;
        assign Sum = m3;

    end else begin : impl_unsupported
        initial begin
            $display("Unsupported implementation for adder_1bit: %d", IMPL_TYPE);
            $finish;
        end
    end
    endgenerate

endmodule
