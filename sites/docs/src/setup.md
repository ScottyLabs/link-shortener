# Setup

## Prerequisites

- [Nix](https://nixos.org/download/) with flakes enabled
- [devenv](https://devenv.sh/getting-started/)
- [direnv](https://direnv.net/)

## Getting started

Log in to OpenBao once per machine. The shell resolves secrets as it loads and will not activate without a token, so run this on a fresh checkout before entering the shell for the first time:

```bash
nix run git+https://codeberg.org/ScottyLabs/devenv#login
```

The token renews on each shell entry, so you only repeat this on a new machine.

Let direnv activate the environment automatically when you enter the directory:

```bash
direnv allow
```

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

## Secrets

Secrets are managed with secretspec and resolved from the ScottyLabs vault. The dev environment wires this up in `devenv.yaml`, so the values are present in the shell and inherited by any process you start from it. The complete set is declared in `secretspec.toml`:

- `KEYCLOAK_URL` and `KEYCLOAK_REALM` locate the Keycloak realm
- `OIDC_CLIENT_ID` and `OIDC_CLIENT_SECRET` are the confidential client credentials
- `OAUTH_RELAY_URL` is the shared OAuth relay callback that Keycloak redirects to

Neither `DATABASE_URL` nor `APP_URL` is a secret, and the deployment platform injects both in production. Locally the devenv PostgreSQL service provides `DATABASE_URL` and `scottylabs.ricochet.appUrl` provides `APP_URL`.

## OIDC client

Authentication uses a confidential Keycloak client. Because the deployment shares one OAuth relay across apps, the client redirect URI is the relay (`OAUTH_RELAY_URL`) rather than the app. The relay records the per request return target in the OAuth state and forwards the authorization code to `{APP_URL}/auth/callback`, so each app receives its own callback without registering a separate redirect URI.
