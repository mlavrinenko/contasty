<?php

namespace App\Calc;

use App\Contracts\Adder;
use App\Util\Logger;

/**
 * Calculator doc block.
 */
class Calculator implements Adder
{
    // running total
    private int $total = 0;

    public function add(int $lhs, int $rhs): int
    {
        return $lhs + $rhs;
    }

    public function describe(): string
    {
        $banner = <<<TXT
        Calculator
        ==========
        TXT;
        return $banner;
    }
}

function helper(string $name): string
{
    # greet the caller
    $multiply = function (int $a, int $b): int {
        return $a * $b;
    };

    return "Hello, {$name}, this is intentionally long enough to truncate past the limit";
}

class CalculatorTest extends TestCase
{
    public function testAdds(): void
    {
        $calc = new Calculator();
        $this->assertSame(3, $calc->add(1, 2));
    }
}
