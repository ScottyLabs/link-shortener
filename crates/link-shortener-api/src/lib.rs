mod auth;
mod error;
mod links;
mod redirect;

pub use auth::OidcConfig;

use axum::{
    Router,
    error_handling::HandleErrorLayer,
    extract::{FromRequestParts, Request, State},
    http::request::Parts,
    response::{IntoResponse, Redirect},
    routing::get,
};
use axum_oidc::{
    AdditionalClaims, EmptyAdditionalClaims, OidcAuthLayer, OidcClient, OidcLoginLayer,
    OidcSession,
    error::MiddlewareError,
    handle_oidc_redirect,
    openidconnect::{ClientId, ClientSecret, CsrfToken, IssuerUrl, Scope, core::CoreGenderClaim},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use link_shortener_store::Store;
use serde::Serialize;
use std::sync::Arc;
use tower::{ServiceBuilder, ServiceExt};
use tower_http::{services::ServeDir, trace::TraceLayer};
use tower_sessions::{
    Expiry, MemoryStore, Session, SessionManagerLayer,
    cookie::{SameSite, time::Duration},
};
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

/// Bridges tower-sessions to the axum-oidc session storage contract
struct SessionWrapper(Session);

impl<S: Send + Sync> FromRequestParts<S> for SessionWrapper {
    type Rejection = <Session as FromRequestParts<S>>::Rejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self(Session::from_request_parts(parts, state).await?))
    }
}

impl<AC: AdditionalClaims> axum_oidc::Session<AC> for SessionWrapper {
    type Error = tower_sessions::session::Error;

    async fn get(&self) -> Result<OidcSession<AC, CoreGenderClaim>, Self::Error> {
        Ok(self.0.get("axum-oidc").await?.unwrap_or_default())
    }

    async fn set(&mut self, value: OidcSession<AC, CoreGenderClaim>) -> Result<(), Self::Error> {
        self.0.insert("axum-oidc", value).await?;
        Ok(())
    }
}

/// OAuth2 state payload carrying the return_to and a CSRF token
#[derive(Serialize)]
struct RelayState<'a> {
    return_to: &'a str,
    csrf: uuid::Uuid,
}

/// Build the base64url state with a random csrf to guard against login CSRF
fn relay_state(return_to: &str) -> String {
    let state = RelayState {
        return_to,
        csrf: uuid::Uuid::new_v4(),
    };
    URL_SAFE_NO_PAD.encode(serde_json::to_vec(&state).expect("serialize relay state"))
}

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "health", description = "Health check"),
        (name = "links", description = "Link management"),
        (name = "auth", description = "Authentication"),
    ),
    info(title = "ScottyLabs Link Shortener", version = "0.1.0")
)]
struct ApiDoc;

pub async fn router(store: Arc<Store>, oidc_config: OidcConfig) -> anyhow::Result<Router> {
    // Session
    let session_store = MemoryStore::default();
    let session_layer = ServiceBuilder::new().layer(
        SessionManagerLayer::new(session_store)
            .with_secure(false)
            .with_same_site(SameSite::Lax)
            .with_expiry(Expiry::OnInactivity(Duration::hours(1))),
    );

    // OIDC
    let issuer_url = IssuerUrl::new(format!(
        "{}/realms/{}",
        oidc_config.keycloak_url, oidc_config.keycloak_realm
    ))
    .expect("valid issuer URL");

    let app_url = oidc_config.app_url.clone();
    let oidc_client = OidcClient::<EmptyAdditionalClaims>::builder()
        .with_default_http_client()
        .with_redirect_url(
            axum::http::Uri::try_from(oidc_config.oauth_relay_url.clone())
                .expect("valid OAUTH_RELAY_URL"),
        )
        .with_client_id(ClientId::new(oidc_config.client_id.clone()))
        .with_client_secret(ClientSecret::new(oidc_config.client_secret.clone()))
        .with_scopes(vec![
            Scope::new("openid".into()),
            Scope::new("email".into()),
            Scope::new("profile".into()),
        ])
        .with_state_generator(move || {
            CsrfToken::new(relay_state(&format!("{}/auth/callback", app_url)))
        })
        .discover(issuer_url)
        .await
        .map_err(|e| anyhow::anyhow!("oidc discovery failed: {e}"))?
        .build();

    tracing::info!("OIDC discovery completed");

    let login_layer = ServiceBuilder::new()
        .layer(HandleErrorLayer::new(|e: MiddlewareError| async move {
            tracing::error!("OIDC login error: {:?}", e);
            e.into_response()
        }))
        .layer(OidcLoginLayer::<EmptyAdditionalClaims, SessionWrapper>::new());

    let auth_layer = ServiceBuilder::new()
        .layer(HandleErrorLayer::new(|e: MiddlewareError| async move {
            tracing::error!("OIDC auth error: {:?}", e);
            e.into_response()
        }))
        .layer(OidcAuthLayer::<EmptyAdditionalClaims, SessionWrapper>::new(
            oidc_client,
        ));

    // Router
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .route("/auth/login", get(login))
        .layer(login_layer)
        // API routes guarded by the CurrentUser extractor
        .routes(utoipa_axum::routes!(links::list_links))
        .routes(utoipa_axum::routes!(links::create_link))
        .routes(utoipa_axum::routes!(links::update_link))
        .routes(utoipa_axum::routes!(links::delete_link))
        .routes(utoipa_axum::routes!(auth::me))
        .routes(utoipa_axum::routes!(health))
        .route(
            "/auth/callback",
            get(handle_oidc_redirect::<EmptyAdditionalClaims, SessionWrapper>),
        )
        .route("/auth/logout", get(logout))
        .layer(auth_layer)
        .layer(session_layer)
        .layer(TraceLayer::new_for_http())
        .with_state(store.clone())
        .split_for_parts();

    let router = router.merge(SwaggerUi::new("/swagger-ui").url("/openapi.json", api));

    // Serve the built SPA in prod when STATIC_DIR is set, falling back to slug redirects
    let app = match std::env::var("STATIC_DIR") {
        Ok(static_dir) => {
            let serve_dir = ServeDir::new(static_dir).append_index_html_on_directories(true);
            router
                .fallback(move |State(store): State<Arc<Store>>, req: Request| {
                    let serve_dir = serve_dir.clone();
                    async move {
                        let path = req.uri().path().to_owned();
                        if let Some(target) = redirect::slug_target(&store, &path).await {
                            return Redirect::temporary(&target).into_response();
                        }
                        serve_dir.oneshot(req).await.into_response()
                    }
                })
                .with_state(store)
        }
        Err(_) => router.fallback(redirect::fallback).with_state(store),
    };

    Ok(app)
}

#[utoipa::path(get, path = "/api/health", tag = "health", responses((status = OK, body = str)))]
async fn health() -> &'static str {
    "OK"
}

/// Login entry point that runs OIDC then returns to the app root
async fn login() -> Redirect {
    Redirect::to("/")
}

/// Clears the session and returns to the app root
async fn logout(session: Session) -> Redirect {
    let _ = session.flush().await;
    Redirect::to("/")
}
