import { createDeltroveClient } from "./client.js";

const environment = (globalThis as typeof globalThis & {
  process?: { env?: Record<string, string | undefined> };
}).process?.env;
const baseUrl = environment?.DELTROVE_BASE_URL ?? "http://127.0.0.1:8080";
const client = createDeltroveClient(baseUrl);
const { data, error, response } = await client.GET("/health/live", {
  signal: AbortSignal.timeout(3000),
});

if (!response.ok || error !== undefined || data?.status !== "alive") {
  throw new Error(`unexpected liveness response (${response.status}): ${JSON.stringify(error ?? data)}`);
}

const missing = await client.GET("/v1/operations/{operation_id}", {
  params: { path: { operation_id: "smoke-missing" } },
  signal: AbortSignal.timeout(3000),
});

if (
  missing.response.status !== 404 ||
  missing.error?.code !== "operation_not_found" ||
  missing.error.status !== 404 ||
  missing.error.retryable !== false
) {
  throw new Error(`unexpected operation lookup problem (${missing.response.status}): ${JSON.stringify(missing.error)}`);
}

console.log("generated TypeScript contract client received alive and typed operation_not_found");
