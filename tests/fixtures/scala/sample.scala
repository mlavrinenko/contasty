package com.example.calc

import scala.collection.mutable

// A small calculator.
object Calc {
  val Banner = "Calculator"

  def add(a: Int, b: Int): Int = {
    a + b
  }

  def square(x: Int): Int = x * x

  class Calculator(seed: Int) {
    def total: Int = seed
  }
}

class CalcSpec {
  def adds(): Unit = {
    assert(Calc.add(1, 2) == 3)
  }
}
