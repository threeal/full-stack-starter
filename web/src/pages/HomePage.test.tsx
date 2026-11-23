import { afterEach, expect, test, vi } from "vitest";
import { render } from "vitest-browser-react";
import FakeWebSocket from "../testing/FakeWebSocket.ts";
import HomePage from "./HomePage.tsx";

afterEach(() => {
  vi.unstubAllGlobals();
});

test("counter button shows and increments the synced count", async () => {
  vi.stubGlobal("WebSocket", FakeWebSocket);
  const screen = await render(<HomePage />);
  const loading = screen.getByRole("button", { name: "count is …" });
  await expect.element(loading).toBeDisabled();

  const webSocket = FakeWebSocket.last;
  webSocket.open();
  await vi.waitFor(() => {
    expect(webSocket.sent).toEqual([{ type: "subscribe", key: "counter" }]);
  });
  webSocket.receive({ type: "sync", key: "counter", version: 0, value: 1 });
  await screen.getByRole("button", { name: "count is 1" }).click();

  await vi.waitFor(() => {
    expect(webSocket.sent).toContainEqual({
      type: "call",
      id: 0,
      method: "counter.increment",
    });
  });
  webSocket.receive({ type: "sync", key: "counter", version: 1, value: 2 });
  webSocket.receive({ type: "result", id: 0, value: null });
  await expect.element(screen.getByText("count is 2")).toBeVisible();
});
