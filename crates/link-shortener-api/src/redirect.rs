use axum::{
    extract::State,
    http::{StatusCode, Uri},
    response::{IntoResponse, Redirect, Response},
};
use link_shortener_store::Store;
use std::sync::Arc;

/// Resolve a request path to a slug target URL when the path is a single
/// segment matching a slug.
pub async fn slug_target(store: &Store, path: &str) -> Option<String> {
    let slug = path.trim_start_matches('/');
    if slug.is_empty() || slug.contains('/') {
        return None;
    }
    match store.links().find_by_slug(slug).await {
        Ok(Some(link)) => Some(link.target_url),
        _ => None,
    }
}

/// Slug redirect fallback. Returns 307 if the path matches a slug, otherwise 404.
pub async fn fallback(State(store): State<Arc<Store>>, uri: Uri) -> Response {
    match slug_target(&store, uri.path()).await {
        Some(target) => Redirect::temporary(&target).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
