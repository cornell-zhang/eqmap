module \$_AND_ (A, B, Y);
  input A, B;
  output Y;
  AND u (.A(A), .B(B), .Y(Y));
endmodule

module \$_OR_ (A, B, Y);
  input A, B;
  output Y;
  OR u (.A(A), .B(B), .Y(Y));
endmodule

module \$_NOT_ (A, Y);
  input A;
  output Y;
  INV u (.A(A), .Y(Y));
endmodule
