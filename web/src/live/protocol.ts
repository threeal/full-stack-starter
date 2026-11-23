/** A message the client sends to the server. */
export type ClientMessage =
  | { type: "call"; id: number; method: string; params: unknown }
  | { type: "subscribe"; key: string }
  | { type: "unsubscribe"; key: string };

/** A message the server sends to the client. */
export type ServerMessage =
  | { type: "result"; id: number; value: unknown }
  | { type: "error"; id: number; code: string; message: string }
  | { type: "sync"; key: string; version: number; value: unknown }
  | { type: "sync_error"; key: string; code: string; message: string };
