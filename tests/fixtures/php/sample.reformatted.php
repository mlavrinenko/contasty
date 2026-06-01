<?php

namespace App\Calc;

class Calculator implements Adder
{
    private int $total = 0;

    public function add(int $lhs, int $rhs): int {}

    public function describe(): string {}
}

function helper(string $name): string {}
