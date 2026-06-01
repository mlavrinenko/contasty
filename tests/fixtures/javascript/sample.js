import { Adder } from "./contracts.js";
import Logger from "./logger.js";

// Running total seed.
const SEED = 0;

export const TABLE = {
  alpha: 1,
  beta: 2,
  gamma: 3,
  delta: 4,
};

export class Calculator {
  constructor() {
    this.total = SEED;
  }

  add(lhs, rhs) {
    return lhs + rhs;
  }

  describe() {
    const banner = `Calculator total=${this.total}`;
    return banner;
  }
}

export function helper(name) {
  const multiply = (a, b) => a * b;
  return `Hello, ${name}, this banner is intentionally long enough to truncate past the configured default string limit of two hundred and fifty six bytes so the truncation marker is emitted in the golden snapshot output here for js.`;
}

describe("Calculator", () => {
  it("adds", () => {
    const calc = new Calculator();
    expect(calc.add(1, 2)).toBe(3);
  });
});
