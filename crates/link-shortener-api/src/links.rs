use crate::auth::{AuthConfig, CurrentUser};
use crate::error::ApiError;
use crate::rules::{self, MatchKind};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use entity::{link_rules, links};
use link_shortener_store::{LinkWithRules, Store};
use rand::RngExt;
use sea_orm::ActiveValue;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use url::Url;
use uuid::Uuid;

#[derive(Deserialize, utoipa::ToSchema)]
pub struct RuleInput {
    kind: MatchKind,
    pattern: String,
    target_url: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct RuleResponse {
    id: Uuid,
    kind: MatchKind,
    pattern: String,
    target_url: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateLinkRequest {
    slug: Option<String>,
    target_url: String,
    #[serde(default)]
    rules: Vec<RuleInput>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateLinkRequest {
    slug: Option<String>,
    target_url: Option<String>,
    rules: Option<Vec<RuleInput>>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct LinkResponse {
    id: Uuid,
    slug: String,
    target_url: String,
    owner_name: Option<String>,
    rules: Vec<RuleResponse>,
    created_at: chrono::NaiveDateTime,
    updated_at: chrono::NaiveDateTime,
}

impl TryFrom<link_rules::Model> for RuleResponse {
    type Error = ApiError;

    fn try_from(m: link_rules::Model) -> Result<Self, Self::Error> {
        let kind = MatchKind::parse(&m.kind).ok_or_else(|| {
            ApiError::Internal(anyhow::anyhow!(
                "rule {} has unknown match kind {:?}",
                m.id,
                m.kind
            ))
        })?;
        Ok(Self {
            id: m.id,
            kind,
            pattern: m.pattern,
            target_url: m.target_url,
        })
    }
}

impl TryFrom<LinkWithRules> for LinkResponse {
    type Error = ApiError;

    fn try_from((link, rules): LinkWithRules) -> Result<Self, Self::Error> {
        Ok(Self {
            id: link.id,
            slug: link.slug,
            target_url: link.target_url,
            owner_name: link.owner_name,
            rules: rules
                .into_iter()
                .map(RuleResponse::try_from)
                .collect::<Result<_, _>>()?,
            created_at: link.created_at,
            updated_at: link.updated_at,
        })
    }
}

/// Admins may modify any link; everyone else only their own, and only while in the project group
fn can_modify(user: &CurrentUser, auth: &AuthConfig, link: &links::Model) -> bool {
    user.is_admin(auth) || (link.owner_id == user.subject && user.in_project_group(auth))
}

fn generate_slug() -> String {
    const CHARSET: &[u8] = b"abcdefghijkmnpqrstuvwxyz23456789";
    let mut rng = rand::rng();
    (0..7)
        .map(|_| CHARSET[rng.random_range(0..CHARSET.len())] as char)
        .collect()
}

/// Reject targets that are not absolute http or https URLs
fn is_http_url(value: &str) -> bool {
    Url::parse(value).is_ok_and(|url| matches!(url.scheme(), "http" | "https") && url.has_host())
}

fn is_rule_target(value: &str) -> bool {
    const BLOCKED: [&str; 5] = ["javascript", "data", "vbscript", "file", "blob"];
    Url::parse(value).is_ok_and(|url| !BLOCKED.contains(&url.scheme()))
}

/// A slug must be a single non-empty path segment to be resolvable
fn is_valid_slug(slug: &str) -> bool {
    !slug.is_empty() && !slug.contains('/')
}

fn rule_models(rules: Vec<RuleInput>) -> Result<Vec<link_rules::ActiveModel>, ApiError> {
    if rules.len() > rules::MAX_RULES {
        return Err(ApiError::BadRequest(format!(
            "a link may have at most {} rules",
            rules::MAX_RULES
        )));
    }

    rules
        .into_iter()
        .map(|rule| {
            rules::validate_pattern(rule.kind, &rule.pattern).map_err(ApiError::BadRequest)?;
            if !is_rule_target(&rule.target_url) {
                return Err(ApiError::BadRequest(format!(
                    "rule target_url {:?} must be an absolute URL with a non-executable scheme",
                    rule.target_url
                )));
            }
            Ok(link_rules::ActiveModel {
                kind: ActiveValue::Set(rule.kind.as_str().to_owned()),
                pattern: ActiveValue::Set(rule.pattern),
                target_url: ActiveValue::Set(rule.target_url),
                ..Default::default()
            })
        })
        .collect()
}

#[utoipa::path(
    get,
    path = "/api/links",
    tag = "links",
    responses((status = OK, body = Vec<LinkResponse>))
)]
pub async fn list_links(
    user: CurrentUser,
    Extension(auth): Extension<Arc<AuthConfig>>,
    State(store): State<Arc<Store>>,
) -> Result<Json<Vec<LinkResponse>>, ApiError> {
    let links = if user.is_admin(&auth) {
        store.links().list_all().await?
    } else {
        store.links().list_by_owner(&user.subject).await?
    };
    Ok(Json(
        links
            .into_iter()
            .map(LinkResponse::try_from)
            .collect::<Result<_, _>>()?,
    ))
}

#[utoipa::path(
    post,
    path = "/api/links",
    tag = "links",
    request_body = CreateLinkRequest,
    responses((status = CREATED, body = LinkResponse))
)]
pub async fn create_link(
    user: CurrentUser,
    Extension(auth): Extension<Arc<AuthConfig>>,
    State(store): State<Arc<Store>>,
    Json(body): Json<CreateLinkRequest>,
) -> Result<(axum::http::StatusCode, Json<LinkResponse>), ApiError> {
    if !user.can_create(&auth) {
        return Err(ApiError::Forbidden);
    }

    if !is_http_url(&body.target_url) {
        return Err(ApiError::BadRequest(
            "target_url must be an http or https URL".into(),
        ));
    }

    let slug = match body.slug {
        Some(slug) => {
            if !is_valid_slug(&slug) {
                return Err(ApiError::BadRequest(
                    "slug must be non-empty and contain no slashes".into(),
                ));
            }
            slug
        }
        None => generate_slug(),
    };

    let rules = rule_models(body.rules)?;

    let link = links::ActiveModel {
        slug: ActiveValue::Set(slug),
        target_url: ActiveValue::Set(body.target_url),
        owner_id: ActiveValue::Set(user.subject),
        owner_name: ActiveValue::Set(Some(user.name)),
        ..Default::default()
    };

    let result = store.links().create(link, rules).await?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(LinkResponse::try_from(result)?),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/links/{id}",
    tag = "links",
    request_body = UpdateLinkRequest,
    responses((status = OK, body = LinkResponse))
)]
pub async fn update_link(
    user: CurrentUser,
    Extension(auth): Extension<Arc<AuthConfig>>,
    State(store): State<Arc<Store>>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateLinkRequest>,
) -> Result<Json<LinkResponse>, ApiError> {
    let existing = store
        .links()
        .find_by_id(id)
        .await?
        .ok_or(ApiError::NotFound)?;

    if !can_modify(&user, &auth, &existing) {
        return Err(ApiError::Forbidden);
    }

    let mut active: links::ActiveModel = existing.into();

    if let Some(slug) = body.slug {
        if !is_valid_slug(&slug) {
            return Err(ApiError::BadRequest(
                "slug must be non-empty and contain no slashes".into(),
            ));
        }
        active.slug = ActiveValue::Set(slug);
    }
    if let Some(target_url) = body.target_url {
        if !is_http_url(&target_url) {
            return Err(ApiError::BadRequest(
                "target_url must be an http or https URL".into(),
            ));
        }
        active.target_url = ActiveValue::Set(target_url);
    }

    let rules = body.rules.map(rule_models).transpose()?;

    let result = store.links().update(active, rules).await?;
    Ok(Json(LinkResponse::try_from(result)?))
}

#[utoipa::path(
    delete,
    path = "/api/links/{id}",
    tag = "links",
    responses((status = NO_CONTENT))
)]
pub async fn delete_link(
    user: CurrentUser,
    Extension(auth): Extension<Arc<AuthConfig>>,
    State(store): State<Arc<Store>>,
    Path(id): Path<Uuid>,
) -> Result<axum::http::StatusCode, ApiError> {
    let existing = store
        .links()
        .find_by_id(id)
        .await?
        .ok_or(ApiError::NotFound)?;

    if !can_modify(&user, &auth, &existing) {
        return Err(ApiError::Forbidden);
    }

    store.links().delete(id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::is_rule_target;

    #[test]
    fn rule_targets_allow_deep_links_but_not_executable_schemes() {
        assert!(is_rule_target("https://apps.apple.com/app/id123"));
        assert!(is_rule_target("myapp://open?ref=1"));
        assert!(is_rule_target("intent://scan#Intent;scheme=zxing;end"));
        assert!(!is_rule_target("javascript:alert(1)"));
        assert!(!is_rule_target("data:text/html,<script>x</script>"));
        assert!(!is_rule_target("/relative/path"));
    }
}
