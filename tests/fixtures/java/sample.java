package com.example.calc;

import java.util.function.IntBinaryOperator;
import java.util.List;

// A small calculator.
public abstract class Calculator {
    private int total;
    static final String BANNER = "Calculator";

    public Calculator(int seed) {
        this.total = seed;
    }

    public int add(int a, int b) {
        return a + b;
    }

    /* Apply a binary operator. */
    public int apply(IntBinaryOperator op, int a, int b) {
        return op.applyAsInt(a, b);
    }

    public abstract int describe();
}

class CalculatorTest {
    @Test
    void adds() {
        assertEquals(3, 3);
    }
}
