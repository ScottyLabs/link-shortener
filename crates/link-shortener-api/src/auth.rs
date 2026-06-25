use axum::{
    Extension, Json,
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};
use axum_oidc::{AdditionalClaims, OidcClaims, openidconnect};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct OidcConfig {
    pub keycloak_url: String,
    pub keycloak_realm: String,
    pub client_id: String,
    pub client_secret: String,
    pub app_url: String,
    pub oauth_relay_url: String,
    pub project_group: String,
    pub project_admin_group: String,
}

/// Keycloak group paths that authorize link management
pub struct AuthConfig {
    pub project_group: String,
    pub project_admin_group: String,
}

/// Group memberships carried on the OIDC token as full paths
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupClaims {
    #[serde(default)]
    pub groups: Vec<String>,
}

impl openidconnect::AdditionalClaims for GroupClaims {}
impl AdditionalClaims for GroupClaims {}

/// Authenticated user extracted from OIDC claims
pub struct CurrentUser {
    pub subject: String,
    pub name: String,
    pub groups: Vec<String>,
}

impl CurrentUser {
    pub fn is_admin(&self, cfg: &AuthConfig) -> bool {
        self.groups.contains(&cfg.project_admin_group)
    }

    pub fn in_project_group(&self, cfg: &AuthConfig) -> bool {
        self.groups.contains(&cfg.project_group)
    }

    pub fn can_create(&self, cfg: &AuthConfig) -> bool {
        self.is_admin(cfg) || self.in_project_group(cfg)
    }
}

impl<S> FromRequestParts<S> for CurrentUser
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let claims = OidcClaims::<GroupClaims>::from_request_parts(parts, state)
            .await
            .map_err(|_| StatusCode::UNAUTHORIZED)?;

        let subject = claims.subject().to_string();
        let name = claims
            .name()
            .and_then(|n| n.get(None))
            .map(|n| n.as_str().to_owned())
            .or_else(|| claims.preferred_username().map(|u| u.as_str().to_owned()))
            .unwrap_or_else(|| subject.clone());
        let groups = claims.additional_claims().groups.clone();

        Ok(CurrentUser {
            subject,
            name,
            groups,
        })
    }
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct UserInfo {
    pub subject: String,
    pub name: String,
    pub can_create: bool,
    pub is_admin: bool,
}

#[utoipa::path(
    get,
    path = "/api/me",
    tag = "auth",
    responses((status = OK, body = UserInfo), (status = UNAUTHORIZED))
)]
pub async fn me(user: CurrentUser, Extension(auth): Extension<Arc<AuthConfig>>) -> Json<UserInfo> {
    Json(UserInfo {
        can_create: user.can_create(&auth),
        is_admin: user.is_admin(&auth),
        name: user.name,
        subject: user.subject,
    })
}
