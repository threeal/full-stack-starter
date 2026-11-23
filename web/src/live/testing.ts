import type { Connect } from "./client.ts";
import type { ClientMessage, ServerMessage } from "./protocol.ts";

/**
 * A fake connection to the server for tests, recording what the client sends
 * and delivering what the server would send.
 */
export class FakeConnection {
  /** The messages the client sent, in order. */
  readonly sent: ClientMessage[] = [];

  readonly #receivers: ((message: ServerMessage) => void)[] = [];

  /**
   * Connects a client to this fake connection.
   * @param receive - Receives the messages delivered by {@link FakeConnection.receive}.
   * @returns A transport that records what the client sends.
   */
  readonly connect: Connect = (receive) => {
    this.#receivers.push(receive);
    return {
      send: (message) => {
        this.sent.push(message);
      },
    };
  };

  /**
   * Delivers a message from the server to the connected client.
   * @param message - The message to deliver.
   */
  receive(message: ServerMessage): void {
    for (const receive of this.#receivers) receive(message);
  }
}
