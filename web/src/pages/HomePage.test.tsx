import { afterEach, expect, test, vi } from "vitest";
import { render } from "vitest-browser-react";
import HomePage from "./HomePage.tsx";

afterEach(() => {
  vi.unstubAllGlobals();
});

test("counter button shows and increments the server's count", async () => {
  const fetch = vi.fn((input: string) =>
    Promise.resolve(Response.json({ count: input === "/api/counter" ? 1 : 2 })),
  );
  vi.stubGlobal("fetch", fetch);
  const screen = await render(<HomePage />);
  await screen.getByRole("button", { name: "count is 1" }).click();
  await expect.element(screen.getByText("count is 2")).toBeVisible();
  expect(fetch).toHaveBeenCalledWith("/api/counter/increment", {
    method: "POST",
  });
});
