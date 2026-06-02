// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "./Ownable.sol";

contract Calculator {
    uint256 public total;

    constructor() {
        total = 0;
    }

    function add(uint256 a, uint256 b) public pure returns (uint256) {
        return a + b;
    }

    function banner() public pure returns (string memory) {
        return "this banner is intentionally long enough to truncate past the configured default string limit of two hundred and fifty six bytes so the truncation marker is emitted in the golden snapshot output for the solidity fixture plus padding appended to comfortably exceed the limit";
    }

    function testAdd() public {
        require(add(1, 2) == 3, "bad");
    }
}
