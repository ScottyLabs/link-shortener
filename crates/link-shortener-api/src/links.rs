use crate::auth::{AuthConfig, CurrentUser};
use crate::error::ApiError;
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use entity::links;
use link_shortener_store::Store;
use rand::RngExt;
use sea_orm::ActiveValue;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use url::Url;
use uuid::Uuid;

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateLinkRequest {
    slug: Option<String>,
    target_url: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateLinkRequest {
    slug: Option<String>,
    target_url: Option<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct LinkResponse {
    id: Uuid,
    slug: String,
    target_url: String,
    owner_name: Option<String>,
    created_at: chrono::NaiveDateTime,
    updated_at: chrono::NaiveDateTime,
}

impl From<links::Model> for LinkResponse {
    fn from(m: links::Model) -> Self {
        Self {
            id: m.id,
            slug: m.slug,
            target_url: m.target_url,
            owner_name: m.owner_name,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
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
    Ok(Json(links.into_iter().map(LinkResponse::from).collect()))
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

    let slug = body.slug.unwrap_or_else(generate_slug);

    let link = links::ActiveModel {
        slug: ActiveValue::Set(slug),
        target_url: ActiveValue::Set(body.target_url),
        owner_id: ActiveValue::Set(user.subject),
        owner_name: ActiveValue::Set(Some(user.name)),
        ..Default::default()
    };

    let result = store.links().create(link).await?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(LinkResponse::from(result)),
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

    let result = store.links().update(active).await?;
    Ok(Json(LinkResponse::from(result)))
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
