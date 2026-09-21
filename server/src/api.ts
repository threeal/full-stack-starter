import { FastifyInstance } from "fastify";

export function apiRoutes(fastify: FastifyInstance) {
  let counter = 0;

  fastify.get("/api/counter", () => {
    return { counter };
  });

  fastify.put("/api/counter", () => {
    ++counter;
    return { counter };
  });
}
