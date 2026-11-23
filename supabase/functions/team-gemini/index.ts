import { handleGemini } from "./handler.ts";

Deno.serve((request) => handleGemini(request, (name) => Deno.env.get(name)));
