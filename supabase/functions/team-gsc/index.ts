import { handleGsc } from "./handler.ts";

Deno.serve((request) => handleGsc(request, (name) => Deno.env.get(name)));
