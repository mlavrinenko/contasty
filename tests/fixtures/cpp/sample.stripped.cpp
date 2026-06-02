namespace calc {

class Calculator {
    int total = 0;
    static const std::vector<int> kSeeds;

public:
    explicit Calculator(int seed) : total(seed) {}

    int add(int a, int b) {}

    int total_value() const {}
};

const std::vector<int> Calculator::kSeeds = {};

auto square = [](int x) {};

}  
