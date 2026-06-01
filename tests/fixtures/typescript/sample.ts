import { Adder } from "./contracts";
import Logger from "./logger";

// Running total seed.
const SEED = 0;

export const TABLE = {
  alpha: 1,
  beta: 2,
  gamma: 3,
  delta: 4,
};

export class Calculator implements Adder {
  private total: number = SEED;

  add(lhs: number, rhs: number): number {
    return lhs + rhs;
  }

  describe(): string {
    const banner = `Calculator total=${this.total}`;
    return banner;
  }
}

export function helper(name: string): string {
  const multiply = (a: number, b: number): number => a * b;
  return `Hello, ${name}, this banner is intentionally long enough to truncate past the configured default string limit of two hundred and fifty six bytes so the truncation marker is emitted in the golden snapshot output here.`;
}

describe("Calculator", () => {
  it("adds", () => {
    const calc = new Calculator();
    expect(calc.add(1, 2)).toBe(3);
  });
});
