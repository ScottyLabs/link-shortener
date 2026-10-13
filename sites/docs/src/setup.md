# Setup

## Getting started

Log in to OpenBao and set up devenv by following kennel's [local development](https://docs.kennel.scottylabs.org/guides/secrets.html#local-development) and [devenv setup](https://docs.kennel.scottylabs.org/guides/deploying.html#2-set-up-devenv) guides.

Start PostgreSQL in the background, and stop it when you are done:

```bash
# Start
devenv up -d

# Stop
devenv processes down
```

Run the backend and frontend in separate terminals:

```bash
# Backend API on port 3000, applies pending migrations on startup
cargo run -p link-shortener

# Frontend dev server on port 5173 with hot reloading
cd sites/web && deno task dev
```

The shell also provides helper commands: `generate-api` regenerates the typed API client from the running backend, `migration NAME` scaffolds a new migration, `migrate` applies pending migrations manually, `generate-entities` regenerates the SeaORM entity code from the database schema, and `docs` serves this documentation locally.
