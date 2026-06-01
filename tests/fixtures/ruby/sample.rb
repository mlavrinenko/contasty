require "set"
require_relative "support"

# A small calculator.
module Calc
  SEEDS = { first: 1, second: 2, third: 3 }

  class Calculator
    BANNER = "Calculator"

    def initialize(seed)
      @total = seed
    end

    def add(a, b)
      a + b
    end

    def self.zero
      new(0)
    end
  end
end

def test_adds
  assert_equal 3, Calc::Calculator.new(0).add(1, 2)
end
