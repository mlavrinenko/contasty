import os
from typing import List

# Running total seed.
SEED = 0

TABLE = {
    "alpha": 1,
    "beta": 2,
    "gamma": 3,
    "delta": 4,
}


class Calculator:
    """Calculator docstring."""

    def __init__(self) -> None:
        self.total = SEED

    def add(self, lhs: int, rhs: int) -> int:
        return lhs + rhs

    def describe(self) -> str:
        banner = "Calculator"
        return banner


def helper(name: str) -> str:
    multiply = lambda a, b: a * b
    return f"Hello, {name}, this banner is intentionally long enough to truncate past the configured default string limit of two hundred and fifty six bytes so the truncation marker is emitted in the golden snapshot output here for python."


def test_adds():
    calc = Calculator()
    assert calc.add(1, 2) == 3


@staticmethod
def test_decorated():
    assert True


class TestCalculator:
    def test_method(self):
        assert True
