# Setup

## Getting started

Install [Lix](https://lix.systems/install/) with the ScottyLabs binary cache:

```bash
curl -sSf -L https://install.lix.systems/lix | sh -s -- install \
  --extra-conf "extra-substituters = https://scottylabs.cachix.org" \
  --extra-conf "extra-trusted-public-keys = scottylabs.cachix.org-1:hajjEX5SLi/Y7yYloiXTt2IOr3towcTGRhMh1vu6Tjg="
```

Then log in to OpenBao by following kennel's [local development](https://docs.kennel.scottylabs.org/guides/secrets.html#local-development) guide, and enter the dev shell with `nix develop`.

Start PostgreSQL and the OAuth relay in the background, and stop them when you are done:

```bash
pitchfork start --local
pitchfork stop --local
```

Run the backend and frontend in separate terminals:

```bash
# Backend API on port 3000, applies pending migrations on startup
cargo run -p link-shortener

# Frontend dev server on port 5173 with hot reloading
cd sites/web && deno task dev
```

The shell also provides helper commands: `generate-api` regenerates the typed API client from the running backend, `migration NAME` scaffolds a new migration, `migrate` applies pending migrations manually, `generate-entities` regenerates the SeaORM entity code from the database schema, and `docs` serves this documentation locally.
