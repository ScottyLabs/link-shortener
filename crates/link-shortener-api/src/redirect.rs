use crate::rules;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode, Uri, header},
    response::{IntoResponse, Redirect, Response},
};
use link_shortener_store::Store;
use std::sync::Arc;

/// Resolve a request path to a slug target URL when the path is a single
/// segment matching a slug.
pub async fn slug_target(store: &Store, path: &str, user_agent: Option<&str>) -> Option<String> {
    let slug = path.trim_start_matches('/');
    if slug.is_empty() || slug.contains('/') {
        return None;
    }
    match store.links().find_by_slug(slug).await {
        Ok(Some((link, link_rules))) => {
            Some(rules::resolve_target(&link, &link_rules, user_agent).to_owned())
        }
        _ => None,
    }
}

pub fn user_agent(headers: &HeaderMap) -> Option<&str> {
    headers.get(header::USER_AGENT)?.to_str().ok()
}

pub fn redirect_response(target: &str) -> Response {
    (
        [(header::VARY, header::USER_AGENT.as_str())],
        Redirect::temporary(target),
    )
        .into_response()
}

/// Slug redirect fallback. Returns 307 if the path matches a slug, otherwise 404.
pub async fn fallback(State(store): State<Arc<Store>>, headers: HeaderMap, uri: Uri) -> Response {
    match slug_target(&store, uri.path(), user_agent(&headers)).await {
        Some(target) => redirect_response(&target),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
