import * as fs from "node:fs";
import * as http from "node:http";
import * as path from "node:path";

export function handleStaticFile(
  publicDir: string,
  req: http.IncomingMessage,
  res: http.ServerResponse,
) {
  const filePath = (req.url === "/" ? null : req.url) ?? "/index.html";
  const contentType =
    path.extname(filePath) === ".html" ? "text/html" : "text/plain";

  fs.readFile(path.join(publicDir, filePath), (err, data) => {
    if (err) {
      res.writeHead(404, { "Content-Type": "text/plain" });
      res.end("404 Not Found");
      return;
    }

    res.writeHead(200, { "Content-Type": contentType });
    res.end(data);
  });
}
