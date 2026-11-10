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
use std::borrow::Cow;
use url::{Url, form_urlencoded};

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

    match store.links().find_by_slug(&slug).await {
        Ok(Some((link, link_rules))) => {
            let rule = rules::matching_rule(link_rules, request.user_agent());
            let target = rule
                .as_ref()
                .map_or(link.target_url.as_str(), |rule| rule.target_url.as_str());

            analytics.link_visited(request, &link, rule.as_ref(), target);

            let target = with_visitor_query(target, request.query());
            Outcome::Redirect(redirect_response(&target))
        }
        Ok(None) => Outcome::Missing(analytics.link_not_found(request, &slug)),
        Err(err) => {
            tracing::error!(%err, slug = &*slug, "slug lookup failed");
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

/// Appends the visitor's query string to the target, dropping any target parameter the visitor also sets so the visitor's value wins. Pairs are copied verbatim to keep each side's percent-encoding intact.
fn with_visitor_query<'a>(target: &'a str, query: Option<&str>) -> Cow<'a, str> {
    let Some(query) = query.filter(|query| !query.is_empty()) else {
        return Cow::Borrowed(target);
    };
    let Ok(mut url) = Url::parse(target) else {
        return Cow::Borrowed(target);
    };

    let overridden: Vec<_> = form_urlencoded::parse(query.as_bytes())
        .map(|(key, _)| key)
        .collect();
    let kept = url
        .query()
        .unwrap_or_default()
        .split('&')
        .filter(|pair| !pair.is_empty())
        .filter(|pair| {
            form_urlencoded::parse(pair.as_bytes())
                .next()
                .is_none_or(|(key, _)| !overridden.contains(&key))
        });
    let merged = kept
        .chain(query.split('&').filter(|pair| !pair.is_empty()))
        .collect::<Vec<_>>()
        .join("&");

    url.set_query(Some(&merged));
    Cow::Owned(url.into())
}

#[cfg(test)]
mod tests {
    use super::with_visitor_query;

    #[test]
    fn visitor_query_is_appended_and_wins_on_conflict() {
        assert_eq!(
            with_visitor_query(
                "https://example.com/?utm_source=cmu.lol&ref=1",
                Some("utm_source=instagram&fbclid=abc")
            ),
            "https://example.com/?ref=1&utm_source=instagram&fbclid=abc"
        );
        assert_eq!(
            with_visitor_query("https://example.com/?q=a%20b", Some("name=J%C3%BCrgen+M")),
            "https://example.com/?q=a%20b&name=J%C3%BCrgen+M"
        );
        assert_eq!(
            with_visitor_query("https://example.com/page#section", Some("a=1")),
            "https://example.com/page?a=1#section"
        );
        assert_eq!(
            with_visitor_query("myapp://open?ref=1", Some("ref=2")),
            "myapp://open?ref=2"
        );
    }
}
