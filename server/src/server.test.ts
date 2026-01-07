import * as http from "node:http";
import { describe, expect, it } from "vitest";
import { httpServerListenToAnyPort } from "./server.js";

describe("httpServerListenToAnyPort", { concurrent: true }, () => {
  it("should return a port number when server listens successfully", async () => {
    const server = http.createServer();
    const port = await httpServerListenToAnyPort(server);

    expect(typeof port).toBe("number");
    expect(port).toBeGreaterThan(0);
    expect(port).toBeLessThanOrEqual(65535);
  });
});
