#!/usr/bin/env node

import * as fs from "node:fs";
import * as http from "node:http";
import * as path from "node:path";

const server = http.createServer((req, res) => {
  const filePath = (req.url === "/" ? null : req.url) ?? "/index.html";
  const contentType =
    path.extname(filePath) === ".html" ? "text/html" : "text/plain";

  fs.readFile(path.join("public", filePath), (err, data) => {
    if (err) {
      res.writeHead(404, { "Content-Type": "text/plain" });
      res.end("404 Not Found");
      return;
    }

    res.writeHead(200, { "Content-Type": contentType });
    res.end(data);
  });
});

server.listen(3000, () => {
  console.log(`Server running at http://localhost:3000`);
});
