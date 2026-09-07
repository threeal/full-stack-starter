import { useState } from "react";
import "./App.css";

type ConnectionStatus = "idle" | "connecting" | "connected" | "error";

export default function App() {
  const [count, setCount] = useState(0);
  const [address, setAddress] = useState("");
  const [status, setStatus] = useState<ConnectionStatus>("idle");

  const checkConnection = async (address: string) => {
    setStatus("connecting");
    try {
      const response = await fetch(new URL("/ping", address));
      setStatus(response.ok ? "connected" : "error");
    } catch {
      setStatus("error");
    }
  };

  if (status !== "connected") {
    return (
      <>
        <form
          onSubmit={(event) => {
            event.preventDefault();
            void checkConnection(address);
          }}
        >
          <input
            type="url"
            placeholder="Backend address"
            value={address}
            disabled={status === "connecting"}
            onChange={(event) => {
              setAddress(event.target.value);
              setStatus("idle");
            }}
          />
          <button type="submit" disabled={status === "connecting"}>
            {status === "connecting" ? "Connecting..." : "Connect"}
          </button>
        </form>
        <p>
          {status === "error"
            ? "Failed to connect to the backend. Please check the address and try again."
            : "Enter the backend address and connect to get started."}
        </p>
      </>
    );
  }

  return (
    <>
      <button
        type="button"
        onClick={() => {
          setCount((count) => count + 1);
        }}
      >
        count is {count}
      </button>
      <p>
        Edit <code>src/App.tsx</code> and save to test HMR
      </p>
    </>
  );
}
