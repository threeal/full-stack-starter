import * as http from "node:http";
import { AddressInfo } from "node:net";

export async function httpServerListenToAnyPort(
  server: http.Server,
): Promise<number> {
  await new Promise<void>((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, resolve);
  });

  const addr = server.address() as AddressInfo;
  return addr.port;
}
