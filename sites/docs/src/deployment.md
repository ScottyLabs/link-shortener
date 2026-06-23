# Deployment

The flake exports the packages the ScottyLabs platform builds and serves. There is no self contained NixOS module; deployment is declared in `devenv.nix` and the platform builds the flake on every push.

## Flake packages

| Attr | Contents |
| ---------------- | -------------------------------------------------------- |
| `link-shortener` | Backend binary with the built SPA baked in via `STATIC_DIR` |
| `web` | The built SPA as static files |
| `docs` | This documentation site, built with mdbook |
| `default` | Alias for `link-shortener` |

Every package is built with the shared scottylabs helpers: `buildRustService` (crane) for the backend, `buildDenoTask` for the SPA, and `buildMdbook` for the docs. The backend is wrapped so `STATIC_DIR` points at the `web` package, which is why the single binary serves both the API and the SPA.

## Deployment configuration

Domains are declared in `devenv.nix` under the scottylabs deployment options:

```nix
scottylabs.kennel.services.link-shortener.customDomain = "cmu.lol";
scottylabs.kennel.sites.docs.customDomain = "docs.cmu.lol";
```

A `services` key must match a flake package attr, which the platform runs as the backend and exposes on its custom domain with TLS. A `sites` key must match a flake package attr, whose built static files are served from that package. Here the `link-shortener` service serves both the API and the SPA, and the `docs` site serves this documentation.

## Secrets

Runtime secrets are declared in `secretspec.toml` and resolved from the ScottyLabs vault at deploy time. The backend reads them as environment variables:

| Variable | Purpose |
| ------------------------------------- | ------------------------------------------------ |
| `KEYCLOAK_URL`, `KEYCLOAK_REALM` | Keycloak realm location |
| `OIDC_CLIENT_ID`, `OIDC_CLIENT_SECRET` | Confidential OIDC client credentials |
| `OAUTH_RELAY_URL` | Shared OAuth relay callback registered with Keycloak |

`DATABASE_URL` and `APP_URL` come from the environment rather than secretspec: the platform injects both at deploy time, while in development the devenv PostgreSQL service provides `DATABASE_URL` and `scottylabs.ricochet.appUrl` provides `APP_URL`. `PORT` defaults to 3000, `RUST_LOG` to a built-in filter, and `STATIC_DIR` is set by the build to the bundled SPA.
