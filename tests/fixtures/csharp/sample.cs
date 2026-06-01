using System;
using System.Collections.Generic;

namespace Demo.Calc;

// A small calculator.
public class Calculator {
    private int total;
    const string Banner = "Calculator";
    static readonly int[] Seeds = { 1, 2, 3 };

    public Calculator(int seed) {
        total = seed;
    }

    public int Add(int a, int b) {
        return a + b;
    }

    /* total accessor */
    public int Total {
        get { return total; }
    }

    public int Square(int x) => x * x;
}

public class CalculatorTests {
    [Fact]
    public void Adds() {
        Assert.Equal(3, 3);
    }
}
