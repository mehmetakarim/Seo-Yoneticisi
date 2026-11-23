import { handleCatalogPublisher } from "./handler.ts";

Deno.serve((request) => handleCatalogPublisher(request, (name) => Deno.env.get(name)));
