import { handleGscCollector } from "../team-gsc/handler.ts";

Deno.serve((request) => handleGscCollector(request, (name) => Deno.env.get(name)));
