import type { Connect } from "./client.ts";
import type { ServerMessage } from "./protocol.ts";

/** How long to wait before reconnecting, in milliseconds. */
export interface Backoff {
  /** The wait after the first failed attempt, doubled after each one. */
  initial: number;
  /** The longest wait. */
  max: number;
}

/**
 * Connects to the server's WebSocket, carrying messages as JSON, and
 * reconnects whenever it closes.
 *
 * Each reconnect waits a random time between half and all of the current
 * wait, which doubles after each attempt up to `backoff.max`, and starts
 * over once a connection opens.
 * @param url - The URL of the server's WebSocket.
 * @param backoff - How long to wait before reconnecting.
 * @returns A function that connects a client to the WebSocket.
 */
export function connectWebSocket(
  url: string,
  backoff: Backoff = { initial: 500, max: 10_000 },
): Connect {
  return (receiver) => {
    let wait = backoff.initial;
    let webSocket: WebSocket;
    const connect = () => {
      webSocket = new WebSocket(url);
      webSocket.addEventListener("open", () => {
        wait = backoff.initial;
        receiver.open();
      });
      webSocket.addEventListener("message", (event: MessageEvent<string>) => {
        receiver.receive(JSON.parse(event.data) as ServerMessage);
      });
      webSocket.addEventListener("close", () => {
        receiver.close();
        setTimeout(connect, wait * (0.5 + Math.random() / 2));
        wait = Math.min(wait * 2, backoff.max);
      });
    };
    connect();
    return {
      send: (message) => {
        webSocket.send(JSON.stringify(message));
      },
    };
  };
}
