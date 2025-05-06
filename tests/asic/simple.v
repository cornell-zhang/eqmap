module simple (
    input wire a,
    input wire b,
    output wire sum,
    output wire carry
);

    // Sum is the XOR of inputs
    assign sum = a ^ b;

    // Carry is the AND of inputs
    assign carry = a & b;

endmodule