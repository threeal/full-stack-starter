import { describe, test, vi } from "vitest";
import { Client, LiveError } from "./client.ts";
import { FakeConnection } from "./testing.ts";

/**
 * Creates a client connected to an open fake connection.
 * @returns The fake connection and the client connected to it.
 */
function connect() {
  const connection = new FakeConnection();
  const client = new Client(connection.connect);
  connection.open();
  return { connection, client };
}

describe("Client", { concurrent: true }, () => {
  test("resolves a call with its result", async ({ expect }) => {
    const { connection, client } = connect();
    const result = client.call("a", { b: 1 });
    expect(connection.sent).toEqual([
      { type: "call", id: 0, method: "a", params: { b: 1 } },
    ]);
    connection.receive({ type: "result", id: 0, value: 42 });
    await expect(result).resolves.toBe(42);
  });

  test("rejects a call with its error", async ({ expect }) => {
    const { connection, client } = connect();
    const result = client.call("a");
    connection.receive({ type: "error", id: 0, code: "c", message: "m" });
    await expect(result).rejects.toThrow(LiveError);
    await expect(result).rejects.toMatchObject({ code: "c", message: "m" });
  });

  test("ignores replies to unknown calls and keys", ({ expect }) => {
    const { connection, client } = connect();
    connection.receive({ type: "result", id: 0, value: 1 });
    connection.receive({ type: "error", id: 0, code: "c", message: "m" });
    connection.receive({ type: "sync", key: "a", version: 0, value: 1 });
    connection.receive({
      type: "sync_error",
      key: "a",
      code: "c",
      message: "m",
    });
    expect(client.value("a")).toBeUndefined();
  });

  test("shares one subscription per key", ({ expect }) => {
    const { connection, client } = connect();
    const unsubscribeFirst = client.subscribe("a", vi.fn());
    const unsubscribeSecond = client.subscribe("a", vi.fn());
    unsubscribeFirst();
    expect(connection.sent).toEqual([{ type: "subscribe", key: "a" }]);
    unsubscribeSecond();
    expect(connection.sent).toEqual([
      { type: "subscribe", key: "a" },
      { type: "unsubscribe", key: "a" },
    ]);
  });

  test("notifies listeners of a synced value", ({ expect }) => {
    const { connection, client } = connect();
    const listener = vi.fn();
    client.subscribe("a", listener);
    expect(client.value("a")).toBeUndefined();
    connection.receive({ type: "sync", key: "a", version: 0, value: 1 });
    expect(listener).toHaveBeenCalledOnce();
    expect(client.value("a")).toBe(1);
  });

  test("throws a rejected subscription's error from its value", ({
    expect,
  }) => {
    const { connection, client } = connect();
    const listener = vi.fn();
    client.subscribe("a", listener);
    connection.receive({
      type: "sync_error",
      key: "a",
      code: "c",
      message: "m",
    });
    expect(listener).toHaveBeenCalledOnce();
    expect(() => client.value("a")).toThrow(LiveError);
  });

  test("sends calls and subscriptions once connected", ({ expect }) => {
    const connection = new FakeConnection();
    const client = new Client(connection.connect);
    void client.call("a");
    client.subscribe("b", vi.fn());
    client.subscribe("c", vi.fn())();
    expect(connection.sent).toEqual([]);
    connection.open();
    expect(connection.sent).toEqual([
      { type: "subscribe", key: "b" },
      { type: "call", id: 0, method: "a" },
    ]);
  });

  test("tells whether it is connected", ({ expect }) => {
    const connection = new FakeConnection();
    const client = new Client(connection.connect);
    const listener = vi.fn();
    const unsubscribe = client.subscribeConnection(listener);
    expect(client.connected).toBe(false);
    connection.open();
    expect(client.connected).toBe(true);
    connection.close();
    expect(client.connected).toBe(false);
    expect(listener).toHaveBeenCalledTimes(2);
    unsubscribe();
    connection.open();
    expect(listener).toHaveBeenCalledTimes(2);
  });

  test("rejects calls cut off by a disconnect", async ({ expect }) => {
    const { connection, client } = connect();
    const result = client.call("a");
    connection.close();
    await expect(result).rejects.toMatchObject({ code: "disconnected" });
    connection.open();
    expect(connection.sent).toEqual([{ type: "call", id: 0, method: "a" }]);
  });

  test("ignores a close while disconnected", ({ expect }) => {
    const connection = new FakeConnection();
    const client = new Client(connection.connect);
    const listener = vi.fn();
    client.subscribeConnection(listener);
    connection.close();
    expect(listener).not.toHaveBeenCalled();
  });

  test("resubscribes on reconnect, keeping the last value", ({ expect }) => {
    const { connection, client } = connect();
    client.subscribe("a", vi.fn());
    connection.receive({ type: "sync", key: "a", version: 3, value: 1 });
    connection.close();
    expect(client.value("a")).toBe(1);
    connection.open();
    expect(connection.sent).toEqual([
      { type: "subscribe", key: "a" },
      { type: "subscribe", key: "a" },
    ]);
    connection.receive({ type: "sync", key: "a", version: 0, value: 2 });
    expect(client.value("a")).toBe(2);
  });
});
