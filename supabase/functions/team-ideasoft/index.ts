import { handleIdeasoft } from "./handler.ts";

Deno.serve((request) => handleIdeasoft(request, (name) => Deno.env.get(name)));
