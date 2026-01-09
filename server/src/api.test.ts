import Fastify, { FastifyInstance } from "fastify";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { apiRoutes } from "./api.js";

describe("apiRoutes", { concurrent: true }, () => {
  let fastify: FastifyInstance;

  beforeAll(async () => {
    fastify = Fastify();
    fastify.register(apiRoutes);
    await fastify.ready();
  });

  describe("GET and PUT /api/counter", { sequential: true }, () => {
    it("returns the initial counter value", async () => {
      const response = await fastify.inject({
        method: "GET",
        url: "/api/counter",
      });

      expect(response.statusCode).toBe(200);
      expect(JSON.parse(response.body)).toEqual({ counter: 0 });
    });

    it("increments the counter", async () => {
      const response = await fastify.inject({
        method: "PUT",
        url: "/api/counter",
      });

      expect(response.statusCode).toBe(200);
      expect(JSON.parse(response.body)).toEqual({ counter: 1 });
    });

    it("returns the updated counter value", async () => {
      const response = await fastify.inject({
        method: "GET",
        url: "/api/counter",
      });

      expect(response.statusCode).toBe(200);
      expect(JSON.parse(response.body)).toEqual({ counter: 1 });
    });
  });

  afterAll(() => fastify.close());
});
