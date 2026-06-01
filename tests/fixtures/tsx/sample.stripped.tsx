
const GREETING = "hi";

interface Props {
  name: string;
}

export function Greeting({ name }: Props): JSX.Element {}

export const Badge = ({ label }: { label: string }): JSX.Element => {};

