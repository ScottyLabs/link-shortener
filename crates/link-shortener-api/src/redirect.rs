use crate::{
    analytics::{Analytics, RequestInfo},
    rules,
};
use axum::{
    http::header,
    response::{IntoResponse, Redirect, Response},
};
use link_shortener_store::Store;
use posthog_rs::Event;

pub enum Outcome {
    Redirect(Response),
    /// Sent only if no static file matches either
    Missing(Option<Event>),
    Skip,
}

pub async fn redirect(store: &Store, analytics: &Analytics, request: &RequestInfo<'_>) -> Outcome {
    let Some(slug) = request.slug() else {
        return Outcome::Skip;
    };

    match store.links().find_by_slug(slug).await {
        Ok(Some((link, link_rules))) => {
            let rule = rules::matching_rule(link_rules, request.user_agent());
            let target = rule
                .as_ref()
                .map_or(link.target_url.as_str(), |rule| rule.target_url.as_str());

            analytics.link_visited(request, &link, rule.as_ref(), target);
            Outcome::Redirect(redirect_response(target))
        }
        Ok(None) => Outcome::Missing(analytics.link_not_found(request, slug)),
        Err(err) => {
            tracing::error!(%err, slug, "slug lookup failed");
            Outcome::Skip
        }
    }
}

fn redirect_response(target: &str) -> Response {
    (
        [(header::VARY, header::USER_AGENT.as_str())],
        Redirect::temporary(target),
    )
        .into_response()
}
