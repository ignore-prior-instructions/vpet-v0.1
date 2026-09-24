import { Field } from "@vpet/ds";

export const Text = () => <Field label="Server" defaultValue="https://vpet.example.com" />;
export const WithHelp = () => (
  <Field label="Pet code" placeholder="lalafu-7f3k" help="Enter the same code on every device." />
);
export const Number = () => <Field label="Clock offset (hours)" type="number" defaultValue={1} />;
