import { describe, expect, test } from "vitest";
import { render } from "vitest-browser-react";
import { Client } from "./client.ts";
import { ClientContext, useClient, useConnected, useSynced } from "./react.ts";
import { FakeConnection } from "./testing.ts";

/**
 * Renders the synced value at `a`.
 * @returns The value as text.
 */
function SyncedValue() {
  return <p>value is {String(useSynced("a"))}</p>;
}

/**
 * Renders whether the client is connected.
 * @returns Whether it is connected, as text.
 */
function Connected() {
  return <p>connected is {String(useConnected())}</p>;
}

/**
 * Renders the message of the error that {@link useClient} throws, if any.
 * @returns The error message as text.
 */
function ClientError() {
  let message = "none";
  try {
    useClient();
  } catch (error) {
    message = (error as Error).message;
  }
  return <p>error is {message}</p>;
}

describe("useSynced", () => {
  test("subscribes while mounted and returns the latest value", async () => {
    const connection = new FakeConnection();
    const client = new Client(connection.connect);
    connection.open();
    const screen = await render(
      <ClientContext value={client}>
        <SyncedValue />
      </ClientContext>,
    );
    await expect.element(screen.getByText("value is undefined")).toBeVisible();
    expect(connection.sent).toEqual([{ type: "subscribe", key: "a" }]);
    connection.receive({ type: "sync", key: "a", version: 0, value: 1 });
    await expect.element(screen.getByText("value is 1")).toBeVisible();
    await screen.unmount();
    expect(connection.sent).toEqual([
      { type: "subscribe", key: "a" },
      { type: "unsubscribe", key: "a" },
    ]);
  });
});

describe("useConnected", () => {
  test("returns whether the client is connected", async () => {
    const connection = new FakeConnection();
    const client = new Client(connection.connect);
    const screen = await render(
      <ClientContext value={client}>
        <Connected />
      </ClientContext>,
    );
    await expect.element(screen.getByText("connected is false")).toBeVisible();
    connection.open();
    await expect.element(screen.getByText("connected is true")).toBeVisible();
  });
});

describe("useClient", () => {
  test("throws without a provided client", async () => {
    const screen = await render(<ClientError />);
    const message = "error is useClient must be used inside a ClientContext";
    await expect.element(screen.getByText(message)).toBeVisible();
  });
});
