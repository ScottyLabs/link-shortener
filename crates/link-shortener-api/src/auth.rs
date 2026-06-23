use axum::{
    Json,
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};
use axum_oidc::{EmptyAdditionalClaims, OidcClaims};
use serde::Serialize;

pub struct OidcConfig {
    pub keycloak_url: String,
    pub keycloak_realm: String,
    pub client_id: String,
    pub client_secret: String,
    pub app_url: String,
    pub oauth_relay_url: String,
}

/// Authenticated user extracted from OIDC claims.
pub struct CurrentUser {
    pub subject: String,
}

impl<S> FromRequestParts<S> for CurrentUser
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let claims = OidcClaims::<EmptyAdditionalClaims>::from_request_parts(parts, state)
            .await
            .map_err(|_| StatusCode::UNAUTHORIZED)?;

        Ok(CurrentUser {
            subject: claims.subject().to_string(),
        })
    }
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct UserInfo {
    pub subject: String,
}

#[utoipa::path(
    get,
    path = "/api/me",
    tag = "auth",
    responses((status = OK, body = UserInfo), (status = UNAUTHORIZED))
)]
pub async fn me(user: CurrentUser) -> Json<UserInfo> {
    Json(UserInfo {
        subject: user.subject,
    })
}
