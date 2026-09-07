import fastifyCors from "@fastify/cors";
import Fastify from "fastify";

const fastify = Fastify({ logger: true });

await fastify.register(fastifyCors, { origin: true });

fastify.get("/ping", () => {
  return { message: "pong" };
});

try {
  await fastify.listen();
} catch (err) {
  fastify.log.error(err);
  process.exitCode = 1;
}
