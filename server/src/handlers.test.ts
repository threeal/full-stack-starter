import { mkdtemp, rm, writeFile } from "node:fs/promises";
import * as http from "node:http";
import * as os from "node:os";
import * as path from "node:path";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { handleStaticFile } from "./handlers.js";

describe("Handle static files", { concurrent: true }, () => {
  let publicDir: string;
  beforeAll(async () => {
    publicDir = await mkdtemp(path.join(os.tmpdir(), "temp"));
    await writeFile(path.join(publicDir, "index.html"), "<!doctype html>");
  });

  let server: http.Server;
  beforeAll(async () => {
    server = new http.Server((req, res) => {
      handleStaticFile(publicDir, req, res);
    });

    await new Promise<void>((resolve) => {
      server.listen(3000, resolve);
    });
  });

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

  afterAll(
    () =>
      new Promise<void>((resolve, reject) => {
        server.close((err) => {
          if (err) {
            reject(err);
          } else {
            resolve();
          }
        });
      }),
  );

  afterAll(() => rm(publicDir, { recursive: true }));
});
