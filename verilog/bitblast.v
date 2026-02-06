module \$_AND_ (A, B, Y);
  input A, B;
  output Y;
  AND u (.A(A), .B(B), .Y(Y));
endmodule

module \$_XOR_ (A, B, Y);
    input A, B;
    output Y;
    wire not_a, not_b, and_a_notb, and_b_nota;
    INV u_not_a (.A(A), .ZN(not_a));
    INV u_not_b (.A(B), .ZN(not_b));
    AND u_and_a (.A(A), .B(not_b), .Y(and_a_notb));
    AND u_and_b (.A(B), .B(not_a), .Y(and_b_nota));
    OR u_or (.A(and_a_notb), .B(and_b_nota), .Y(Y));
endmodule

module \$_MUX_ (A, B, S, Y);
    input S, A, B;
    output Y;
    wire not_s, and_a_nots, and_b_s;
    INV u_not_s (.A(S), .ZN(not_s));
    AND u_and_a (.A(A), .B(not_s), .Y(and_a_nots));
    AND u_and_b (.A(B), .B(S), .Y(and_b_s));
    OR u_or (.A(and_a_nots), .B(and_b_s), .Y(Y));
endmodule

module \$_OR_ (A, B, Y);
  input A, B;
  output Y;
  OR u (.A(A), .B(B), .Y(Y));
endmodule

module \$_NOT_ (A, Y);
  input A;
  output Y;
  INV u (.A(A), .ZN(Y));
endmodule
