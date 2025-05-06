module simple (
    a,
    b,
    sum,
);
  input a;
  wire a;
  input b;
  wire b;
  output sum;
  wire sum;

  XOR _01_ (
      .A(a),
      .B(b),
      .Y(sum)
  );

endmodule