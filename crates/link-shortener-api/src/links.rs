use crate::analytics::{Analytics, LinkChange, RequestMeta};
use crate::auth::{AuthConfig, CurrentUser};
use crate::error::ApiError;
use crate::rules::{self, MatchKind};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use chrono::{DateTime, Utc};
use entity::{link_rules, links};
use link_shortener_store::{LinkDetails, Store, StoreError};
use rand::RngExt;
use sea_orm::ActiveValue;
use serde::{Deserialize, Deserializer, Serialize};
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
    /// Case-insensitive
    slug: Option<String>,
    target_url: String,
    #[serde(default)]
    rules: Vec<RuleInput>,
    #[serde(default)]
    paused: bool,
    expires_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateLinkRequest {
    /// Case-insensitive
    slug: Option<String>,
    target_url: Option<String>,
    rules: Option<Vec<RuleInput>>,
    paused: Option<bool>,
    /// Omit to keep the current expiry, or send null to remove it
    #[serde(default, deserialize_with = "present")]
    #[schema(value_type = Option<DateTime<Utc>>)]
    expires_at: Option<Option<DateTime<Utc>>>,
}

/// Distinguishes an explicit null, `Some(None)`, from an omitted field, `None`
fn present<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(deserializer).map(Some)
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct LinkResponse {
    id: Uuid,
    slug: String,
    target_url: String,
    owner_name: Option<String>,
    rules: Vec<RuleResponse>,
    paused: bool,
    expires_at: Option<DateTime<Utc>>,
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

impl TryFrom<LinkDetails> for LinkResponse {
    type Error = ApiError;

    fn try_from(LinkDetails { link, owner, rules }: LinkDetails) -> Result<Self, Self::Error> {
        Ok(Self {
            id: link.id,
            slug: link.slug,
            target_url: link.target_url,
            owner_name: owner.and_then(|owner| owner.name),
            rules: rules
                .into_iter()
                .map(RuleResponse::try_from)
                .collect::<Result<_, _>>()?,
            paused: link.paused,
            expires_at: link.expires_at.map(|expires_at| expires_at.and_utc()),
            created_at: link.created_at,
            updated_at: link.updated_at,
        })
    }
}

/// Admins may modify any link, and everyone else only their own
fn can_modify(user: &CurrentUser, auth: &AuthConfig, link: &links::Model) -> bool {
    user.is_admin(auth) || link.owner_id == user.subject
}

const GENERATED_SLUG_RETRIES: usize = 4;

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

/// Slugs are a single path segment, and dots are reserved for static files like `/favicon.ico`
pub(crate) fn is_valid_slug(slug: &str) -> bool {
    !slug.is_empty() && !slug.contains(['/', '.'])
}

const INVALID_SLUG: &str = "slug must be non-empty and contain no slashes or dots";

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
    request: RequestMeta,
    Extension(analytics): Extension<Arc<Analytics>>,
    State(store): State<Arc<Store>>,
    Json(body): Json<CreateLinkRequest>,
) -> Result<(axum::http::StatusCode, Json<LinkResponse>), ApiError> {
    if !is_http_url(&body.target_url) {
        return Err(ApiError::BadRequest(
            "target_url must be an http or https URL".into(),
        ));
    }

    let generated = body.slug.is_none();
    let mut slug = match body.slug {
        Some(slug) => {
            if !is_valid_slug(&slug) {
                return Err(ApiError::BadRequest(INVALID_SLUG.into()));
            }
            slug
        }
        None => generate_slug(),
    };

    let rules = rule_models(body.rules)?;

    store.users().upsert(&user.subject, &user.name).await?;

    let mut retries = 0;
    let result = loop {
        let link = links::ActiveModel {
            slug: ActiveValue::Set(slug.clone()),
            target_url: ActiveValue::Set(body.target_url.clone()),
            owner_id: ActiveValue::Set(user.subject.clone()),
            paused: ActiveValue::Set(body.paused),
            expires_at: ActiveValue::Set(body.expires_at.map(|expires_at| expires_at.naive_utc())),
            ..Default::default()
        };

        match store.links().create(link, rules.clone()).await {
            Err(StoreError::SlugConflict(_)) if generated && retries < GENERATED_SLUG_RETRIES => {
                retries += 1;
                slug = generate_slug();
            }
            result => break result?,
        }
    };

    let rule_count = result.rules.len();

    analytics.link_changed(
        LinkChange::Created { rule_count },
        &request.info(),
        &user.subject,
        Some(&user.name),
        &result.link,
    );

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
    request: RequestMeta,
    Extension(auth): Extension<Arc<AuthConfig>>,
    Extension(analytics): Extension<Arc<Analytics>>,
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

    let slug_changed = body
        .slug
        .as_ref()
        .is_some_and(|slug| *slug != existing.slug);
    let target_changed = body
        .target_url
        .as_ref()
        .is_some_and(|target| *target != existing.target_url);
    let mut active: links::ActiveModel = existing.into();

    if let Some(slug) = body.slug {
        if !is_valid_slug(&slug) {
            return Err(ApiError::BadRequest(INVALID_SLUG.into()));
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
    if let Some(paused) = body.paused {
        active.paused = ActiveValue::Set(paused);
    }
    if let Some(expires_at) = body.expires_at {
        active.expires_at = ActiveValue::Set(expires_at.map(|expires_at| expires_at.naive_utc()));
    }

    let rules = body.rules.map(rule_models).transpose()?;

    let rule_count = rules.as_ref().map(Vec::len);
    let result = store.links().update(active, rules).await?;

    analytics.link_changed(
        LinkChange::Updated {
            rule_count,
            slug_changed,
            target_changed,
        },
        &request.info(),
        &user.subject,
        Some(&user.name),
        &result.link,
    );
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
    request: RequestMeta,
    Extension(auth): Extension<Arc<AuthConfig>>,
    Extension(analytics): Extension<Arc<Analytics>>,
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

    analytics.link_changed(
        LinkChange::Deleted,
        &request.info(),
        &user.subject,
        Some(&user.name),
        &existing,
    );
    Ok(axum::http::StatusCode::NO_CONTENT)
}
