import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { connectWebSocket } from "./websocket.ts";

class FakeWebSocket extends EventTarget {
  static last: FakeWebSocket;
  readonly url: string;
  readonly sent: unknown[] = [];

  constructor(url: string) {
    super();
    this.url = url;
    FakeWebSocket.last = this;
  }

  send(data: string): void {
    this.sent.push(JSON.parse(data));
  }

  open(): void {
    this.dispatchEvent(new Event("open"));
  }

  receive(message: unknown): void {
    this.dispatchEvent(
      new MessageEvent("message", { data: JSON.stringify(message) }),
    );
  }
}

beforeEach(() => {
  vi.stubGlobal("WebSocket", FakeWebSocket);
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("connectWebSocket", () => {
  test("sends messages as JSON once open", async () => {
    const transport = connectWebSocket("/api/ws")(vi.fn());
    const webSocket = FakeWebSocket.last;
    expect(webSocket.url).toBe("/api/ws");
    transport.send({ type: "subscribe", key: "a" });
    await Promise.resolve();
    expect(webSocket.sent).toEqual([]);
    webSocket.open();
    await vi.waitFor(() => {
      expect(webSocket.sent).toEqual([{ type: "subscribe", key: "a" }]);
    });
  });

  test("delivers messages parsed from JSON", () => {
    const receive = vi.fn();
    connectWebSocket("/api/ws")(receive);
    const message = { type: "sync", key: "a", version: 0, value: 1 };
    FakeWebSocket.last.receive(message);
    expect(receive).toHaveBeenCalledWith(message);
  });
});
