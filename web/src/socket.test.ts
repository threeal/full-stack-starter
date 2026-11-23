import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { Socket, SocketError } from "./socket.ts";
import FakeWebSocket from "./testing/FakeWebSocket.ts";

beforeEach(() => {
  vi.stubGlobal("WebSocket", FakeWebSocket);
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("Socket", () => {
  test("sends messages only once connected", async () => {
    const socket = new Socket("/api/ws");
    const webSocket = FakeWebSocket.last;
    expect(webSocket.url).toBe("/api/ws");
    void socket.call("a");
    await Promise.resolve();
    expect(webSocket.sent).toEqual([]);
    webSocket.open();
    await vi.waitFor(() => {
      expect(webSocket.sent).toEqual([{ type: "call", id: 0, method: "a" }]);
    });
  });

  test("resolves a call with its result", async () => {
    const socket = new Socket("/api/ws");
    const webSocket = FakeWebSocket.last;
    webSocket.open();
    const result = socket.call("a", { b: 1 });
    await vi.waitFor(() => {
      expect(webSocket.sent).toEqual([
        { type: "call", id: 0, method: "a", params: { b: 1 } },
      ]);
    });
    webSocket.receive({ type: "result", id: 0, value: 42 });
    await expect(result).resolves.toBe(42);
  });

  test("rejects a call with its error", async () => {
    const socket = new Socket("/api/ws");
    const webSocket = FakeWebSocket.last;
    const result = socket.call("a");
    webSocket.receive({ type: "error", id: 0, code: "c", message: "m" });
    await expect(result).rejects.toThrow(SocketError);
    await expect(result).rejects.toMatchObject({ code: "c", message: "m" });
  });

  test("ignores replies to unknown calls and keys", () => {
    const socket = new Socket("/api/ws");
    const webSocket = FakeWebSocket.last;
    webSocket.receive({ type: "result", id: 0, value: 1 });
    webSocket.receive({ type: "error", id: 0, code: "c", message: "m" });
    webSocket.receive({ type: "sync", key: "a", version: 0, value: 1 });
    webSocket.receive({
      type: "sync_error",
      key: "a",
      code: "c",
      message: "m",
    });
    expect(socket.value("a")).toBeUndefined();
  });

  test("shares one subscription per key", async () => {
    const socket = new Socket("/api/ws");
    const webSocket = FakeWebSocket.last;
    webSocket.open();
    const unsubscribeFirst = socket.subscribe("a", vi.fn());
    const unsubscribeSecond = socket.subscribe("a", vi.fn());
    unsubscribeFirst();
    await vi.waitFor(() => {
      expect(webSocket.sent).toEqual([{ type: "subscribe", key: "a" }]);
    });
    unsubscribeSecond();
    await vi.waitFor(() => {
      expect(webSocket.sent).toEqual([
        { type: "subscribe", key: "a" },
        { type: "unsubscribe", key: "a" },
      ]);
    });
  });

  test("notifies listeners of a synced value", () => {
    const socket = new Socket("/api/ws");
    const webSocket = FakeWebSocket.last;
    const listener = vi.fn();
    socket.subscribe("a", listener);
    expect(socket.value("a")).toBeUndefined();
    webSocket.receive({ type: "sync", key: "a", version: 0, value: 1 });
    expect(listener).toHaveBeenCalledOnce();
    expect(socket.value("a")).toBe(1);
  });

  test("throws a rejected subscription's error from its value", () => {
    const socket = new Socket("/api/ws");
    const webSocket = FakeWebSocket.last;
    const listener = vi.fn();
    socket.subscribe("a", listener);
    webSocket.receive({
      type: "sync_error",
      key: "a",
      code: "c",
      message: "m",
    });
    expect(listener).toHaveBeenCalledOnce();
    expect(() => socket.value("a")).toThrow(SocketError);
  });
});
