# Setup

## Getting started

Install Lix and log in to OpenBao by following kennel's [local development](https://docs.kennel.scottylabs.org/guides/local-development/#installing-nix) and [secrets](https://docs.kennel.scottylabs.org/guides/secrets/#local-development) guides, then enter the dev shell with `nix develop`.

Start PostgreSQL, Valkey, and the OAuth relay in the background:

```bash
pitchfork start --local
```

Run the backend and frontend in separate terminals:

```bash
# Backend API on port 3000, applies pending migrations on startup
cargo run -p link-shortener

# Frontend dev server on port 5173 with hot reloading
cd sites/web && deno task dev
```

The shell also provides these helper commands:

- `generate-api` regenerates the typed API client from the running backend.
- `migration NAME` scaffolds a new migration.
- `migrate` applies pending migrations manually.
- `generate-entities` regenerates the SeaORM entity code from the database schema.
- `docs` serves this documentation locally.
