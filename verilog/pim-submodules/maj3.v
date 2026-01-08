// Maj3 Submodule
// Dependencies: None
// deyuan, 06/13/2025

module maj3 (
    input A,
    input B,
    input C,
    output Y
);

    assign Y = (A & B) | (A & C) | (B & C);

endmodule
