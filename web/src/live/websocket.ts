import type { Connect } from "./client.ts";
import type { ServerMessage } from "./protocol.ts";

/**
 * Connects to the server's WebSocket, carrying messages as JSON and sending
 * them once the connection opens.
 * @param url - The URL of the server's WebSocket.
 * @returns A function that connects a client to the WebSocket.
 */
export function connectWebSocket(url: string): Connect {
  return (receive) => {
    const webSocket = new WebSocket(url);
    const opened = new Promise<void>((resolve) => {
      webSocket.addEventListener("open", () => {
        resolve();
      });
    });
    webSocket.addEventListener("message", (event: MessageEvent<string>) => {
      receive(JSON.parse(event.data) as ServerMessage);
    });
    return {
      send: (message) => {
        void opened.then(() => {
          webSocket.send(JSON.stringify(message));
        });
      },
    };
  };
}
