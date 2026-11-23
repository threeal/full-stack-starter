import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { RouterProvider } from "react-router";
import { Client, ClientContext, connectWebSocket } from "./live/index.ts";
import { router } from "./router.tsx";

const root = document.getElementById("root");
if (root) {
  const client = new Client(connectWebSocket("/api/ws"));
  createRoot(root).render(
    <StrictMode>
      <ClientContext value={client}>
        <RouterProvider router={router} />
      </ClientContext>
    </StrictMode>,
  );
}
