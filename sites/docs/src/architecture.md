# Architecture

## Crate structure

```
crates/
  link-shortener/         Entry point, wires everything together
  link-shortener-api/     Axum routes, OIDC auth, OpenAPI via utoipa
  link-shortener-store/   Sea-ORM repository layer
  entity/                 Generated Sea-ORM entity models (do not edit)
  migration/              Sea-ORM database migrations
```

## Auth flow

1. The user clicks "Log in" on the frontend, which links to `/auth/login`.
1. `/auth/login` sits behind `OidcLoginLayer`, so axum-oidc redirects the browser to Keycloak. The redirect URI registered with the client is the shared OAuth relay, and the OAuth state carries a CSRF token plus the app callback to return to (`{APP_URL}/auth/callback`).
1. The user authenticates with Keycloak.
1. Keycloak redirects to the OAuth relay with an authorization code.
1. The relay reads the return target from the state and forwards the code to `{APP_URL}/auth/callback`.
1. `/auth/callback` exchanges the code for tokens and creates a server side session.
1. The browser returns to `/auth/login`, which now sees the session and redirects to the app root.
1. The frontend calls `/api/me` and receives user info from the session.

Logging out is a `GET /auth/logout` that clears the session and redirects to the app root. The Keycloak SSO session is left intact, so signing back in does not require re-entering credentials.

## API design

Routes are registered with `utoipa-axum` for automatic OpenAPI schema generation. The schema is served at `/openapi.json` and is browsable at `/swagger-ui`.

API routes require a valid session. The `CurrentUser` extractor returns `401` when the session is missing, so the SPA can detect auth state over XHR without being redirected:

- `GET /api/me`
- `GET /api/links`
- `POST /api/links`
- `PATCH /api/links/{id}`
- `DELETE /api/links/{id}`

Auth routes drive the OIDC flow:

- `GET /auth/login` (behind `OidcLoginLayer`; forces login, then redirects to the app root)
- `GET /auth/logout` (clears the session, then redirects to the app root)
- `GET /auth/callback` (OIDC redirect handler)

Public routes do not require authentication:

- `GET /api/health`
- `GET /{slug}` (slug redirect, handled by the router fallback)

## Static files and slug redirects

In production the backend serves the built SPA itself. When `STATIC_DIR` is set, a `ServeDir` serves the static files and uses the slug handler as its not found service. The slug handler checks whether the path is a single segment matching a slug in the database; if so it returns a 307 redirect, otherwise a 404. When `STATIC_DIR` is unset, as in local development, the SPA is served by the Vite dev server and the backend fallback is the slug handler alone.

## Frontend

The frontend is built with Svelte 5 and Vite on the Deno toolchain. Auth state is determined by calling `/api/me` on mount. The Vite dev server proxies `/api`, `/auth`, `/swagger-ui`, and `/openapi.json` to the backend on port 3000.

Once the backend is running, `deno task generate-api` generates TypeScript types from the OpenAPI schema via `openapi-typescript`. The `openapi-fetch` library provides a typed client for making API calls.
