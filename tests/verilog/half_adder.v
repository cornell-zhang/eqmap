// Verilog
// half_adder
// Ninputs 2
// Noutputs 2
// NtotalGates 2
// xor 1
// and 1

module half_adder ( a,b,s,c);
input a,b;  // Inputs
output s,c;  // Outputs
xor gate_xor (s, a, b);  // XOR gate for sum
and gate_and (c, a, b);  // AND gate for carry
endmodule