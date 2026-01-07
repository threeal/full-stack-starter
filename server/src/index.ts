import fastifyStatic from "@fastify/static";
import Fastify from "fastify";

const fastify = Fastify({ logger: true });

if (process.env.PUBLIC_DIR) {
  fastify.register(fastifyStatic, {
    root: process.env.PUBLIC_DIR,
    prefix: "/",
  });
} else {
  fastify.log.warn("PUBLIC_DIR is not set. Static file handling is disabled.");
}

try {
  await fastify.listen();
} catch (err) {
  fastify.log.error(err);
  process.exitCode = 1;
}
