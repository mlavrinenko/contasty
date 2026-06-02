package com.example.calc;


public abstract class Calculator {
    private int total;
    static final String BANNER = "Calculator";

    public Calculator(int seed) {}

    public int add(int a, int b) {}

    public int apply(IntBinaryOperator op, int a, int b) {}

    public abstract int describe();
}

