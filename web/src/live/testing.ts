import type { Connect, Receiver } from "./client.ts";
import type { ClientMessage, ServerMessage } from "./protocol.ts";

/**
 * A fake connection to the server for tests, recording what the client sends
 * and delivering what the server would send.
 */
export class FakeConnection {
  /** The messages the client sent, in order. */
  readonly sent: ClientMessage[] = [];

  readonly #receivers: Receiver[] = [];

  /**
   * Connects a client to this fake connection, closed until
   * {@link FakeConnection.open} is called.
   * @param receiver - Told what {@link FakeConnection.receive},
   * {@link FakeConnection.open}, and {@link FakeConnection.close} do.
   * @returns A transport that records what the client sends.
   */
  readonly connect: Connect = (receiver) => {
    this.#receivers.push(receiver);
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
    for (const receiver of this.#receivers) receiver.receive(message);
  }

  /** Opens the connection, or reopens it after closing. */
  open(): void {
    for (const receiver of this.#receivers) receiver.open();
  }

  /** Closes the connection. */
  close(): void {
    for (const receiver of this.#receivers) receiver.close();
  }
}
