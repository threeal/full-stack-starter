import type { ClientMessage, ServerMessage } from "./protocol.ts";

/** A way for a client to send messages to the server. */
export interface Transport {
  /** Sends a message to the server. */
  send: (message: ClientMessage) => void;
}

/** Connects a client to the server, delivering the server's messages to `receive`. */
export type Connect = (receive: (message: ServerMessage) => void) => Transport;

/** An error the server replied with to a call or a subscription. */
export class LiveError extends Error {
  /** A machine-readable code identifying the kind of error. */
  readonly code: string;

  /**
   * @param code - A machine-readable code identifying the kind of error.
   * @param message - A human-readable description of the error.
   */
  constructor(code: string, message: string) {
    super(message);
    this.code = code;
  }
}

/** A call waiting for its reply. */
interface PendingCall {
  resolve: (value: unknown) => void;
  reject: (error: LiveError) => void;
}

/** A synced object's subscription, shared by all of its listeners. */
interface Subscription {
  listeners: Set<() => void>;
  value?: unknown;
  error?: LiveError;
}

/**
 * A client of the server's calls and synced objects, independent of how its
 * messages are transported.
 */
export class Client {
  readonly #transport: Transport;
  readonly #calls = new Map<number, PendingCall>();
  readonly #subscriptions = new Map<string, Subscription>();
  #nextCallId = 0;

  /**
   * @param connect - Connects the client to the server.
   */
  constructor(connect: Connect) {
    this.#transport = connect((message) => {
      this.#receive(message);
    });
  }

  /**
   * Calls a method on the server.
   * @param method - The name of the method.
   * @param params - The method's parameters.
   * @returns A promise that resolves to the call's result, or rejects with
   * the server's error.
   */
  call(method: string, params?: unknown): Promise<unknown> {
    const id = this.#nextCallId++;
    return new Promise((resolve, reject) => {
      this.#calls.set(id, { resolve, reject });
      this.#transport.send({ type: "call", id, method, params });
    });
  }

  /**
   * Subscribes to a synced object, sharing one subscription per key.
   * @param key - The synced object's key.
   * @param listener - Called whenever the object's value changes.
   * @returns A function that removes the listener, unsubscribing once no
   * listener is left.
   */
  subscribe(key: string, listener: () => void): () => void {
    const subscription =
      this.#subscriptions.get(key) ?? this.#addSubscription(key);
    subscription.listeners.add(listener);
    return () => {
      subscription.listeners.delete(listener);
      if (subscription.listeners.size === 0) {
        this.#subscriptions.delete(key);
        this.#transport.send({ type: "unsubscribe", key });
      }
    };
  }

  /**
   * Returns the latest whole value of a subscribed synced object.
   * @param key - The synced object's key.
   * @returns The latest value, or `undefined` if none has arrived yet.
   * @throws The server's error if it rejected the subscription.
   */
  value(key: string): unknown {
    const subscription = this.#subscriptions.get(key);
    if (subscription?.error) throw subscription.error;
    return subscription?.value;
  }

  #addSubscription(key: string): Subscription {
    const subscription = { listeners: new Set<() => void>() };
    this.#subscriptions.set(key, subscription);
    this.#transport.send({ type: "subscribe", key });
    return subscription;
  }

  #receive(message: ServerMessage): void {
    switch (message.type) {
      case "result":
        this.#takeCall(message.id)?.resolve(message.value);
        break;
      case "error":
        this.#takeCall(message.id)?.reject(
          new LiveError(message.code, message.message),
        );
        break;
      case "sync":
        this.#updateSubscription(message.key, { value: message.value });
        break;
      case "sync_error":
        this.#updateSubscription(message.key, {
          error: new LiveError(message.code, message.message),
        });
        break;
    }
  }

  #takeCall(id: number): PendingCall | undefined {
    const call = this.#calls.get(id);
    this.#calls.delete(id);
    return call;
  }

  #updateSubscription(
    key: string,
    update: Pick<Subscription, "value" | "error">,
  ): void {
    const subscription = this.#subscriptions.get(key);
    if (subscription === undefined) return;
    Object.assign(subscription, update);
    for (const listener of subscription.listeners) listener();
  }
}
