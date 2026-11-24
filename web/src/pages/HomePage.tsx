import Button from "../components/Button.tsx";
import Description from "../components/Description.tsx";
import { useClient, useConnected, useSynced } from "../live/index.ts";

/**
 * Renders the home page, with a button that shows the server's synced count
 * and increments it when clicked while connected.
 * @returns The home page content.
 */
export default function HomePage() {
  const client = useClient();
  const connected = useConnected();
  const count = useSynced("counter") as number | undefined;
  return (
    <>
      <Description>Click the button to increase the counter.</Description>
      <Button
        disabled={!connected || count === undefined}
        onClick={() => {
          void client.call("counter.increment");
        }}
      >
        count is {count ?? "…"}
      </Button>
    </>
  );
}
