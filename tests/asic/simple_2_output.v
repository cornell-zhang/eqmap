module simple (
    a,
    b,
    sum,
    carry
);
  input a;
  wire a;
  input b;
  wire b;
  output sum;
  wire sum;
  output carry;
  wire carry;

  XOR _01_ (
      .A(a),
      .B(b),
      .Y(sum)
  );

  AND _02_ (
      .A(a),
      .B(b),
      .Y(carry)
  );

endmodule