import { useEffect, useState } from "react";
import Button from "../components/Button.tsx";
import Description from "../components/Description.tsx";

/**
 * Requests the counter API and returns the count it responds with.
 * @param input - The API path to request.
 * @param init - The request options, such as its method.
 * @returns The count in the response.
 */
async function fetchCount(input: string, init?: RequestInit): Promise<number> {
  const response = await fetch(input, init);
  const { count } = (await response.json()) as { count: number };
  return count;
}

/**
 * Renders the home page, with a button that shows the server's count and
 * increments it when clicked.
 * @returns The home page content.
 */
export default function HomePage() {
  const [count, setCount] = useState<number>();
  useEffect(() => {
    void fetchCount("/api/counter").then(setCount);
  }, []);
  return (
    <>
      <Description>Click the button to increase the counter.</Description>
      <Button
        disabled={count === undefined}
        onClick={() => {
          void fetchCount("/api/counter/increment", { method: "POST" }).then(
            setCount,
          );
        }}
      >
        count is {count ?? "…"}
      </Button>
    </>
  );
}
