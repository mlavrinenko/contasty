package com.example.calc

import kotlin.math.abs

// A small calculator.
const val BANNER = "Calculator"

class Calculator(private val seed: Int) {
    var total = seed

    fun add(a: Int, b: Int): Int {
        return a + b
    }

    fun square(x: Int) = x * x

    fun reset() {
        total = seed
    }
}

@Test
fun adds() {
    assertEquals(3, Calculator(0).add(1, 2))
}
