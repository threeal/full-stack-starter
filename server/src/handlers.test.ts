import { mkdtemp, rm, writeFile } from "node:fs/promises";
import * as http from "node:http";
import * as os from "node:os";
import * as path from "node:path";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { handleStaticFile } from "./handlers.js";
import { httpServerListenToAnyPort } from "./server.js";

describe("handleStaticFile", { concurrent: true }, () => {
  let publicDir: string;
  beforeAll(async () => {
    publicDir = await mkdtemp(path.join(os.tmpdir(), "temp"));
    await writeFile(path.join(publicDir, "index.html"), "<!doctype html>");
  });

  let url: string;
  beforeAll(async () => {
    const server = new http.Server((req, res) => {
      handleStaticFile(publicDir, req, res);
    });

    const port = await httpServerListenToAnyPort(server);
    url = `http://localhost:${port.toString()}`;
  });

  it("should serve root path", async () => {
    const res = await fetch(url);
    expect(res.status).toBe(200);
    expect(res.headers.get("content-type")).toBe("text/html");
  });

  it("should serve index.html", async () => {
    const res = await fetch(`${url}/index.html`);
    expect(res.status).toBe(200);
    expect(res.headers.get("content-type")).toBe("text/html");
  });

  it("should return 404 for non-existent files", async () => {
    const res = await fetch(`${url}/nonexistent`);
    expect(res.status).toBe(404);
    expect(res.headers.get("content-type")).toBe("text/plain");
  });

  afterAll(() => rm(publicDir, { recursive: true }));
});
