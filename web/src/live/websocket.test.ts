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

  close(): void {
    this.dispatchEvent(new Event("close"));
  }

  receive(message: unknown): void {
    this.dispatchEvent(
      new MessageEvent("message", { data: JSON.stringify(message) }),
    );
  }
}

/**
 * Creates a receiver that records what the connection tells it.
 * @returns The receiver, made of mock functions.
 */
function fakeReceiver() {
  return { receive: vi.fn(), open: vi.fn(), close: vi.fn() };
}

/**
 * Closes the current WebSocket and expects a new one exactly `wait`
 * milliseconds later.
 * @param wait - The expected wait before reconnecting.
 */
function expectReconnectAfter(wait: number): void {
  const closed = FakeWebSocket.last;
  closed.close();
  vi.advanceTimersByTime(wait - 1);
  expect(FakeWebSocket.last).toBe(closed);
  vi.advanceTimersByTime(1);
  expect(FakeWebSocket.last).not.toBe(closed);
}

beforeEach(() => {
  vi.stubGlobal("WebSocket", FakeWebSocket);
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe("connectWebSocket", () => {
  test("sends messages as JSON", () => {
    const transport = connectWebSocket("/api/ws")(fakeReceiver());
    const webSocket = FakeWebSocket.last;
    expect(webSocket.url).toBe("/api/ws");
    transport.send({ type: "subscribe", key: "a" });
    expect(webSocket.sent).toEqual([{ type: "subscribe", key: "a" }]);
  });

  test("delivers messages parsed from JSON", () => {
    const receiver = fakeReceiver();
    connectWebSocket("/api/ws")(receiver);
    const message = { type: "sync", key: "a", version: 0, value: 1 };
    FakeWebSocket.last.receive(message);
    expect(receiver.receive).toHaveBeenCalledWith(message);
  });

  test("tells when the connection opens and closes", () => {
    const receiver = fakeReceiver();
    connectWebSocket("/api/ws")(receiver);
    FakeWebSocket.last.open();
    expect(receiver.open).toHaveBeenCalledOnce();
    FakeWebSocket.last.close();
    expect(receiver.close).toHaveBeenCalledOnce();
  });

  test("reconnects with a doubling wait that resets once open", () => {
    vi.spyOn(Math, "random").mockReturnValue(1);
    connectWebSocket("/api/ws", { initial: 100, max: 300 })(fakeReceiver());
    expectReconnectAfter(100);
    expectReconnectAfter(200);
    expectReconnectAfter(300);
    expectReconnectAfter(300);
    FakeWebSocket.last.open();
    expectReconnectAfter(100);
  });

  test("waits at least half of the wait before reconnecting", () => {
    vi.spyOn(Math, "random").mockReturnValue(0);
    connectWebSocket("/api/ws", { initial: 100, max: 300 })(fakeReceiver());
    expectReconnectAfter(50);
  });
});
