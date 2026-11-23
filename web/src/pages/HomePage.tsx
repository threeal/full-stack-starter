import Button from "../components/Button.tsx";
import Description from "../components/Description.tsx";
import { getSocket, useSynced } from "../socket.ts";

/**
 * Renders the home page, with a button that shows the server's synced count
 * and increments it when clicked.
 * @returns The home page content.
 */
export default function HomePage() {
  const count = useSynced("counter") as number | undefined;
  return (
    <>
      <Description>Click the button to increase the counter.</Description>
      <Button
        disabled={count === undefined}
        onClick={() => {
          void getSocket().call("counter.increment");
        }}
      >
        count is {count ?? "…"}
      </Button>
    </>
  );
}
