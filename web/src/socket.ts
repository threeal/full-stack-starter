import { useCallback, useSyncExternalStore } from "react";

/** A message the client sends to the server. */
type ClientMessage =
  | { type: "call"; id: number; method: string; params: unknown }
  | { type: "subscribe"; key: string }
  | { type: "unsubscribe"; key: string };

/** A message the server sends to the client. */
type ServerMessage =
  | { type: "result"; id: number; value: unknown }
  | { type: "error"; id: number; code: string; message: string }
  | { type: "sync"; key: string; version: number; value: unknown }
  | { type: "sync_error"; key: string; code: string; message: string };

/** An error the server replied with to a call or a subscription. */
export class SocketError extends Error {
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
  reject: (error: SocketError) => void;
}

/** A synced object's subscription, shared by all of its listeners. */
interface Subscription {
  listeners: Set<() => void>;
  value?: unknown;
  error?: SocketError;
}

/** A WebSocket connection to the server, for calls and synced objects. */
export class Socket {
  readonly #webSocket: WebSocket;
  readonly #opened: Promise<void>;
  readonly #calls = new Map<number, PendingCall>();
  readonly #subscriptions = new Map<string, Subscription>();
  #nextCallId = 0;

  /**
   * @param url - The URL of the server's WebSocket.
   */
  constructor(url: string) {
    this.#webSocket = new WebSocket(url);
    this.#opened = new Promise((resolve) => {
      this.#webSocket.addEventListener("open", () => {
        resolve();
      });
    });
    this.#webSocket.addEventListener(
      "message",
      (event: MessageEvent<string>) => {
        this.#receive(JSON.parse(event.data) as ServerMessage);
      },
    );
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
      this.#send({ type: "call", id, method, params });
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
   * Returns the latest value of a subscribed synced object.
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
    void this.#opened.then(() => {
      this.#webSocket.send(JSON.stringify(message));
    });
  }

  #receive(message: ServerMessage): void {
    switch (message.type) {
      case "result":
        this.#takeCall(message.id)?.resolve(message.value);
        break;
      case "error":
        this.#takeCall(message.id)?.reject(
          new SocketError(message.code, message.message),
        );
        break;
      case "sync":
        this.#updateSubscription(message.key, { value: message.value });
        break;
      case "sync_error":
        this.#updateSubscription(message.key, {
          error: new SocketError(message.code, message.message),
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

let pageSocket: Socket | undefined;

/**
 * Returns the page's connection to the server, connecting on first use.
 * @returns The page's socket.
 */
export function getSocket(): Socket {
  pageSocket ??= new Socket("/api/ws");
  return pageSocket;
}

/**
 * Returns the latest value of a synced object, subscribing to it while the
 * component is mounted.
 * @param key - The synced object's key.
 * @returns The latest value, or `undefined` until the first one arrives.
 */
export function useSynced(key: string): unknown {
  const socket = getSocket();
  const subscribe = useCallback(
    (onChange: () => void) => socket.subscribe(key, onChange),
    [socket, key],
  );
  return useSyncExternalStore(subscribe, () => socket.value(key));
}
