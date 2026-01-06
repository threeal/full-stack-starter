import { describe, expect, it } from "vitest";
import "./index.js";

describe("HTTP server", () => {
  it("should serve root path", async () => {
    const res = await fetch("http://localhost:3000/");
    expect(res.status).toBe(200);
    expect(res.headers.get("content-type")).toBe("text/html");
  });

  it("should serve index.html", async () => {
    const res = await fetch("http://localhost:3000/index.html");
    expect(res.status).toBe(200);
    expect(res.headers.get("content-type")).toBe("text/html");
  });

  it("should return 404 for non-existent files", async () => {
    const res = await fetch("http://localhost:3000/nonexistent");
    expect(res.status).toBe(404);
    expect(res.headers.get("content-type")).toBe("text/plain");
  });
});
