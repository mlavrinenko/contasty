defmodule Calculator do
  @moduledoc "Sums integers."
  alias Calculator.Helper
  import Integer

  # Add two integers.
  def add(a, b) do
    a + b
  end

  def banner do
    "this banner is intentionally long enough to truncate past the configured default string limit of two hundred and fifty six bytes so the truncation marker is emitted in the golden snapshot output for the elixir fixture and here is some extra padding text appended to comfortably exceed the limit"
  end
end

defmodule CalculatorTest do
  use ExUnit.Case

  test "adds" do
    assert Calculator.add(1, 2) == 3
  end
end
