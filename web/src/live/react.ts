import { createContext, use, useCallback, useSyncExternalStore } from "react";
import type { Client } from "./client.ts";

/** Provides the page's client to {@link useClient} and the hooks that use it. */
export const ClientContext = createContext<Client | undefined>(undefined);

/**
 * Returns the client provided by the nearest {@link ClientContext}.
 * @returns The provided client.
 * @throws If no client is provided.
 */
export function useClient(): Client {
  const client = use(ClientContext);
  if (client === undefined) {
    throw new Error("useClient must be used inside a ClientContext");
  }
  return client;
}

/**
 * Returns the latest whole value of a synced object, subscribing to it while
 * the component is mounted.
 * @param key - The synced object's key.
 * @returns The latest value, or `undefined` until the first one arrives.
 */
export function useSynced(key: string): unknown {
  const client = useClient();
  const subscribe = useCallback(
    (onChange: () => void) => client.subscribe(key, onChange),
    [client, key],
  );
  return useSyncExternalStore(subscribe, () => client.value(key));
}

/**
 * Returns whether the connection to the server is open, updating as it opens
 * and closes.
 * @returns Whether the connection is open.
 */
export function useConnected(): boolean {
  const client = useClient();
  return useSyncExternalStore(
    client.subscribeConnection,
    () => client.connected,
  );
}
