use crate::rules::contains_ci;
use axum::{
    extract::FromRequestParts,
    http::{HeaderMap, HeaderName, Method, Uri, header, request::Parts},
};
use entity::{link_rules, links};
use posthog_rs::{Client, ClientOptionsBuilder, Event};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{convert::Infallible, net::IpAddr};
use url::Url;

// posthog-js CAMPAIGN_PARAMS
const ATTRIBUTION_PARAMS: [&str; 25] = [
    "utm_source",
    "utm_medium",
    "utm_campaign",
    "utm_content",
    "utm_term",
    "gad_source",
    "mc_cid",
    "gclid",
    "gclsrc",
    "dclid",
    "gbraid",
    "wbraid",
    "fbclid",
    "msclkid",
    "twclid",
    "li_fat_id",
    "igshid",
    "ttclid",
    "rdt_cid",
    "epik",
    "qclid",
    "sccid",
    "oppref",
    "irclid",
    "_kx",
];

// inapp-spy's user-agent markers, lowercase and matched in order
const IN_APP_BROWSERS: [(&str, &str); 18] = [
    ("fban/messenger", "messenger"),
    ("fb_iab/messenger", "messenger"),
    ("instagram", "instagram"),
    ("fban/", "facebook"),
    ("fbav/", "facebook"),
    ("fb_iab/", "facebook"),
    ("twitter", "twitter"),
    (" line/", "line"),
    ("micromessenger/", "wechat"),
    ("barcelona", "threads"),
    ("musical_ly", "tiktok"),
    ("bytedance", "tiktok"),
    ("snapchat", "snapchat"),
    ("linkedinapp", "linkedin"),
    ("gsa/", "google"),
    ("waios/", "whatsapp"),
    ("wa4a/", "whatsapp"),
    ("reddit/", "reddit"),
];

const CF_CONNECTING_IP: HeaderName = HeaderName::from_static("cf-connecting-ip");
const CF_VISITOR: HeaderName = HeaderName::from_static("cf-visitor");
const SEC_FETCH_SITE: HeaderName = HeaderName::from_static("sec-fetch-site");
const SEC_PURPOSE: HeaderName = HeaderName::from_static("sec-purpose");
const PURPOSE: HeaderName = HeaderName::from_static("purpose");
const SEC_GPC: HeaderName = HeaderName::from_static("sec-gpc");
const X_REQUESTED_WITH: HeaderName = HeaderName::from_static("x-requested-with");

pub struct Analytics {
    client: Option<Client>,
    app_version: Option<String>,
}

impl Analytics {
    pub async fn new(
        api_key: Option<String>,
        host: Option<String>,
        app_version: Option<String>,
    ) -> anyhow::Result<Self> {
        let Some(api_key) = api_key.filter(|key| !key.is_empty()) else {
            return Ok(Self::disabled());
        };

        let mut options = ClientOptionsBuilder::default();
        options.api_key(api_key);
        if let Some(host) = host.filter(|host| !host.is_empty()) {
            options.host(host);
        }

        Ok(Self {
            client: Some(posthog_rs::client(options.build()?).await),
            app_version,
        })
    }

    pub fn disabled() -> Self {
        Self {
            client: None,
            app_version: None,
        }
    }

    pub async fn shutdown(&self) {
        if let Some(client) = &self.client {
            client.shutdown().await;
        }
    }

    pub fn capture(&self, event: Event) {
        if let Some(client) = &self.client {
            client.capture(event);
        }
    }

    pub fn link_visited(
        &self,
        request: &RequestInfo<'_>,
        link: &links::Model,
        rule: Option<&link_rules::Model>,
        target: &str,
    ) {
        let Some(mut event) = self.visitor_event("link_visited", request) else {
            return;
        };

        set(&mut event, "slug", &link.slug);
        set(&mut event, "link_id", link.id);
        set(&mut event, "owner_name", &link.owner_name);
        set(&mut event, "target_url", target);
        set(&mut event, "target_domain", domain(target));
        set(
            &mut event,
            "link_age_days",
            (chrono::Utc::now().naive_utc() - link.created_at).num_days(),
        );
        if let Some(rule) = rule {
            set(&mut event, "rule_id", rule.id);
            set(&mut event, "rule_kind", &rule.kind);
            set(&mut event, "rule_pattern", &rule.pattern);
        }

        self.capture(event);
    }

    pub fn link_not_found(&self, request: &RequestInfo<'_>, slug: &str) -> Option<Event> {
        let mut event = self.visitor_event("link_not_found", request)?;
        set(&mut event, "slug", slug);
        Some(event)
    }

    pub fn link_changed(
        &self,
        change: LinkChange,
        request: &RequestInfo<'_>,
        subject: &str,
        name: Option<&str>,
        link: &links::Model,
    ) {
        if self.client.is_none() {
            return;
        }

        let mut event = Event::new(change.event_name(), subject);
        request.insert_props(&mut event);
        self.insert_app_version(&mut event);
        if let Some(name) = name {
            set(&mut event, "$set", serde_json::json!({ "name": name }));
        }
        set(&mut event, "link_id", link.id);
        set(&mut event, "slug", &link.slug);
        set(&mut event, "target_url", &link.target_url);
        set(&mut event, "target_domain", domain(&link.target_url));
        match change {
            LinkChange::Created { rule_count } => set(&mut event, "rule_count", rule_count),
            LinkChange::Updated {
                rule_count,
                slug_changed,
                target_changed,
            } => {
                set(&mut event, "rule_count", rule_count);
                set(&mut event, "slug_changed", slug_changed);
                set(&mut event, "target_changed", target_changed);
            }
            LinkChange::Deleted => {}
        }

        self.capture(event);
    }

    fn visitor_event(&self, name: &str, request: &RequestInfo<'_>) -> Option<Event> {
        self.client.as_ref()?;

        let mut event = Event::new(name.to_owned(), request.visitor_id());
        set(&mut event, "$process_person_profile", false);
        request.insert_props(&mut event);
        self.insert_app_version(&mut event);
        Some(event)
    }

    fn insert_app_version(&self, event: &mut Event) {
        if let Some(version) = &self.app_version {
            set(event, "$app_version", version);
        }
    }
}

#[derive(Clone, Copy)]
pub enum LinkChange {
    Created {
        rule_count: usize,
    },
    Updated {
        rule_count: Option<usize>,
        slug_changed: bool,
        target_changed: bool,
    },
    Deleted,
}

impl LinkChange {
    const fn event_name(self) -> &'static str {
        match self {
            Self::Created { .. } => "link_created",
            Self::Updated { .. } => "link_updated",
            Self::Deleted => "link_deleted",
        }
    }
}

pub struct RequestInfo<'a> {
    method: &'a Method,
    uri: &'a Uri,
    headers: &'a HeaderMap,
}

impl<'a> RequestInfo<'a> {
    pub const fn new(method: &'a Method, uri: &'a Uri, headers: &'a HeaderMap) -> Self {
        Self {
            method,
            uri,
            headers,
        }
    }

    pub fn slug(&self) -> Option<&'a str> {
        let slug = self.uri.path().trim_start_matches('/');
        (!slug.is_empty() && !slug.contains('/')).then_some(slug)
    }

    pub fn user_agent(&self) -> Option<&'a str> {
        header_str(self.headers, &header::USER_AGENT)
    }

    fn visitor_id(&self) -> String {
        let ip = client_ip(self.headers).map(|ip| ip.to_string());
        let mut hasher = Sha256::new();
        for part in [ip.as_deref(), self.host(), self.user_agent()] {
            hasher.update(part.unwrap_or_default());
            hasher.update([0]);
        }
        format!("{:x}", hasher.finalize())
    }

    fn host(&self) -> Option<&'a str> {
        header_str(self.headers, &header::HOST).or_else(|| self.uri.host())
    }

    fn insert_props(&self, event: &mut Event) {
        if let Some(ip) = client_ip(self.headers) {
            set(event, "$ip", ip.to_string());
        }
        if let Some(user_agent) = self.user_agent() {
            set(event, "$raw_user_agent", user_agent);
            if let Some(app) = in_app_browser(user_agent) {
                set(event, "in_app_browser", app);
            }
        }
        if let Some(host) = self.host() {
            set(event, "$host", host);
            set(
                event,
                "$current_url",
                format!(
                    "{}://{host}{}",
                    scheme(self.headers),
                    self.uri.path_and_query().map_or("/", |pq| pq.as_str())
                ),
            );
        }
        set(event, "$pathname", self.uri.path());
        if let Some(referrer) = header_str(self.headers, &header::REFERER) {
            set(event, "$referrer", referrer);
            if let Some(domain) = domain(referrer) {
                set(event, "$referring_domain", domain);
            }
        }
        if let Some(language) = browser_language(self.headers) {
            set(event, "$browser_language", language);
        }
        if let Some(fetch_site) = header_str(self.headers, &SEC_FETCH_SITE) {
            set(event, "fetch_site", fetch_site);
        }
        if let Some(package) =
            header_str(self.headers, &X_REQUESTED_WITH).filter(|value| *value != "XMLHttpRequest")
        {
            set(event, "in_app_package", package);
        }
        set(event, "is_prefetch", is_prefetch(self.method, self.headers));
        set(
            event,
            "gpc",
            header_str(self.headers, &SEC_GPC).is_some_and(|value| value.trim() == "1"),
        );
        for (key, value) in attribution(self.uri.query()) {
            set(event, key, value);
        }
    }
}

pub struct RequestMeta {
    method: Method,
    uri: Uri,
    headers: HeaderMap,
}

impl RequestMeta {
    pub const fn info(&self) -> RequestInfo<'_> {
        RequestInfo::new(&self.method, &self.uri, &self.headers)
    }
}

impl<S: Send + Sync> FromRequestParts<S> for RequestMeta {
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self {
            method: parts.method.clone(),
            uri: parts.uri.clone(),
            headers: parts.headers.clone(),
        })
    }
}

fn set(event: &mut Event, key: &str, value: impl Serialize) {
    if let Err(err) = event.insert_prop(key, value) {
        tracing::warn!(%err, key, "dropped analytics property");
    }
}

fn header_str<'a>(headers: &'a HeaderMap, name: &HeaderName) -> Option<&'a str> {
    headers
        .get(name)?
        .to_str()
        .ok()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

/// `X-Forwarded-For` only holds the tunnel's address behind Caddy
fn client_ip(headers: &HeaderMap) -> Option<IpAddr> {
    header_str(headers, &CF_CONNECTING_IP)?.parse().ok()
}

fn scheme(headers: &HeaderMap) -> &'static str {
    match header_str(headers, &CF_VISITOR) {
        Some(visitor) if visitor.contains("\"https\"") => "https",
        _ => "http",
    }
}

fn domain(url: &str) -> Option<String> {
    Url::parse(url).ok()?.host_str().map(str::to_owned)
}

fn browser_language(headers: &HeaderMap) -> Option<&str> {
    header_str(headers, &header::ACCEPT_LANGUAGE)?
        .split(',')
        .next()?
        .split(';')
        .next()
        .map(str::trim)
        .filter(|tag| !tag.is_empty() && *tag != "*")
}

fn in_app_browser(user_agent: &str) -> Option<&'static str> {
    IN_APP_BROWSERS
        .iter()
        .find(|(marker, _)| contains_ci(user_agent, marker))
        .map(|(_, app)| *app)
}

fn is_prefetch(method: &Method, headers: &HeaderMap) -> bool {
    *method == Method::HEAD
        || [&SEC_PURPOSE, &PURPOSE]
            .into_iter()
            .filter_map(|name| header_str(headers, name))
            .any(|value| value.to_ascii_lowercase().contains("prefetch"))
}

fn attribution(query: Option<&str>) -> impl Iterator<Item = (&'static str, String)> {
    url::form_urlencoded::parse(query.unwrap_or_default().as_bytes()).filter_map(|(key, value)| {
        let param = ATTRIBUTION_PARAMS.iter().find(|param| **param == key)?;
        (!value.is_empty()).then(|| (*param, value.into_owned()))
    })
}
