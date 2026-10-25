import createClient from "openapi-fetch";
import type { paths } from "./generated/api.js";

export function createDeltroveClient(baseUrl: string) {
  return createClient<paths>({ baseUrl });
}
