/** A fake `WebSocket` for tests, recording what's sent and delivering what's received. */
export default class FakeWebSocket extends EventTarget {
  /** The most recently created fake. */
  static last: FakeWebSocket;

  /** The URL it was created with. */
  readonly url: string;

  /** The messages sent through it, parsed from JSON. */
  readonly sent: unknown[] = [];

  /**
   * @param url - The URL to connect to.
   */
  constructor(url: string) {
    super();
    this.url = url;
    FakeWebSocket.last = this;
  }

  /**
   * Records a message as sent.
   * @param data - The message, serialized to JSON.
   */
  send(data: string): void {
    this.sent.push(JSON.parse(data));
  }

  /** Opens the connection. */
  open(): void {
    this.dispatchEvent(new Event("open"));
  }

  /**
   * Delivers a message from the server.
   * @param message - The message, serialized to JSON before delivery.
   */
  receive(message: unknown): void {
    this.dispatchEvent(
      new MessageEvent("message", { data: JSON.stringify(message) }),
    );
  }
}
