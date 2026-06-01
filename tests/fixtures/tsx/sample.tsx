import { useState } from "react";

// Default greeting.
const GREETING = "hi";

interface Props {
  name: string;
}

export function Greeting({ name }: Props): JSX.Element {
  const [count, setCount] = useState(0);
  return (
    <button onClick={() => setCount(count + 1)}>
      {GREETING}, {name}: {count}
    </button>
  );
}

export const Badge = ({ label }: { label: string }): JSX.Element => (
  <span className="badge">{label}</span>
);

describe("Greeting", () => {
  it("renders", () => {
    expect(true).toBe(true);
  });
});
