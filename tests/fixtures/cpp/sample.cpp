#include <string>
#include <vector>

namespace calc {

// A small calculator.
class Calculator {
    int total = 0;
    static const std::vector<int> kSeeds;

public:
    explicit Calculator(int seed) : total(seed) {}

    int add(int a, int b) {
        return a + b;
    }

    /* const accessor */
    int total_value() const {
        return total;
    }
};

const std::vector<int> Calculator::kSeeds = { 1, 2, 3 };

auto square = [](int x) {
    return x * x;
};

}  // namespace calc

TEST(CalculatorTest, Adds) {
    EXPECT_EQ(calc::Calculator(0).add(1, 2), 3);
}
