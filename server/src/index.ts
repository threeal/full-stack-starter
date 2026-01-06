#!/usr/bin/env node

import * as http from "node:http";
import { handleStaticFile } from "./handlers.js";

const publicDir = process.env.PUBLIC_DIR;
if (!publicDir) {
  console.warn("PUBLIC_DIR is not set. Static file handling is disabled.");
}

const server = http.createServer((req, res) => {
  if (publicDir) {
    handleStaticFile(publicDir, req, res);
  } else {
    res.writeHead(501, { "Content-Type": "text/plain" });
    res.end("501 Not Implemented");
  }
});

server.listen(3000, () => {
  console.log(`Server running at http://localhost:3000`);
});
