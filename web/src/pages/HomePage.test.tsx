import { expect, test } from "vitest";
import { render } from "vitest-browser-react";
import { Client, ClientContext } from "../live/index.ts";
import { FakeConnection } from "../live/testing.ts";
import HomePage from "./HomePage.tsx";

test("counter button shows and increments the synced count", async () => {
  const connection = new FakeConnection();
  const client = new Client(connection.connect);
  connection.open();
  const screen = await render(
    <ClientContext value={client}>
      <HomePage />
    </ClientContext>,
  );
  const loading = screen.getByRole("button", { name: "count is …" });
  await expect.element(loading).toBeDisabled();
  expect(connection.sent).toEqual([{ type: "subscribe", key: "counter" }]);

  connection.receive({ type: "sync", key: "counter", version: 0, value: 1 });
  await screen.getByRole("button", { name: "count is 1" }).click();
  expect(connection.sent).toContainEqual({
    type: "call",
    id: 0,
    method: "counter.increment",
  });

  connection.receive({ type: "sync", key: "counter", version: 1, value: 2 });
  connection.receive({ type: "result", id: 0, value: null });
  await expect.element(screen.getByText("count is 2")).toBeVisible();
});

test("counter button is disabled while disconnected", async () => {
  const connection = new FakeConnection();
  const client = new Client(connection.connect);
  connection.open();
  const screen = await render(
    <ClientContext value={client}>
      <HomePage />
    </ClientContext>,
  );
  connection.receive({ type: "sync", key: "counter", version: 0, value: 1 });
  const button = screen.getByRole("button", { name: "count is 1" });
  await expect.element(button).toBeEnabled();
  connection.close();
  await expect.element(button).toBeDisabled();
});
