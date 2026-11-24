import type { ClientMessage, ServerMessage } from "./protocol.ts";

/** A way for a client to send messages to the server. */
export interface Transport {
  /** Sends a message to the server, only called while the connection is open. */
  send: (message: ClientMessage) => void;
}

/** What a connection tells the client about the server. */
export interface Receiver {
  /** Delivers a message from the server. */
  receive: (message: ServerMessage) => void;
  /** Tells that the connection opened, or reopened after closing. */
  open: () => void;
  /** Tells that the connection closed. */
  close: () => void;
}

/** Connects a client to the server, telling `receiver` what happens on it. */
export type Connect = (receiver: Receiver) => Transport;

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
  message: ClientMessage;
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
 *
 * Calls made while disconnected wait for the connection to open. Calls cut
 * off by a disconnect reject with the code `"disconnected"` instead of being
 * retried, since the server may already have run them. Subscriptions are
 * sent again on every reopen, keeping their last value in the meantime.
 */
export class Client {
  readonly #transport: Transport;
  readonly #calls = new Map<number, PendingCall>();
  readonly #subscriptions = new Map<string, Subscription>();
  readonly #connectionListeners = new Set<() => void>();
  #connected = false;
  #nextCallId = 0;

  /**
   * @param connect - Connects the client to the server.
   */
  constructor(connect: Connect) {
    this.#transport = connect({
      receive: (message) => {
        this.#receive(message);
      },
      open: () => {
        this.#open();
      },
      close: () => {
        this.#close();
      },
    });
  }

  /**
   * Whether the connection to the server is open.
   * @returns Whether the connection is open.
   */
  get connected(): boolean {
    return this.#connected;
  }

  /**
   * Listens to the connection opening and closing.
   * @param listener - Called whenever {@link Client.connected} changes.
   * @returns A function that removes the listener.
   */
  readonly subscribeConnection = (listener: () => void): (() => void) => {
    this.#connectionListeners.add(listener);
    return () => {
      this.#connectionListeners.delete(listener);
    };
  };

  /**
   * Calls a method on the server, once the connection is open.
   * @param method - The name of the method.
   * @param params - The method's parameters.
   * @returns A promise that resolves to the call's result, or rejects with
   * the server's error, or with the code `"disconnected"` if the connection
   * closed before the reply.
   */
  call(method: string, params?: unknown): Promise<unknown> {
    const id = this.#nextCallId++;
    return new Promise((resolve, reject) => {
      const message: ClientMessage = { type: "call", id, method, params };
      this.#calls.set(id, { message, resolve, reject });
      this.#send(message);
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
        this.#send({ type: "unsubscribe", key });
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
    this.#send({ type: "subscribe", key });
    return subscription;
  }

  #send(message: ClientMessage): void {
    if (this.#connected) this.#transport.send(message);
  }

  #open(): void {
    this.#setConnected(true);
    for (const key of this.#subscriptions.keys()) {
      this.#transport.send({ type: "subscribe", key });
    }
    for (const call of this.#calls.values()) {
      this.#transport.send(call.message);
    }
  }

  #close(): void {
    if (!this.#connected) return;
    this.#setConnected(false);
    for (const [id, call] of this.#calls) {
      this.#calls.delete(id);
      call.reject(new LiveError("disconnected", "the connection closed"));
    }
  }

  #setConnected(connected: boolean): void {
    this.#connected = connected;
    for (const listener of this.#connectionListeners) listener();
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
