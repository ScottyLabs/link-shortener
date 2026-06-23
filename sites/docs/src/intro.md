# Introduction

A demo link shortener for ScottyLabs, showcasing the full deployment stack:

- Rust backend with Axum, Sea-ORM, and utoipa (OpenAPI)
- Keycloak OIDC authentication via axum-oidc through the shared ScottyLabs OAuth relay
- Svelte 5 frontend with Vite and openapi-fetch, built with the Deno toolchain
- PostgreSQL 18 with pg_uuidv7
- Nix flake built with the shared scottylabs helpers (crane for the backend, a Deno task builder for the web app, mdbook for docs) and deployed by the ScottyLabs platform

Authenticated users can create, manage, and delete short links. Anyone can follow a short link to its target URL.
