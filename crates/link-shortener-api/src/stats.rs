use crate::auth::{AuthConfig, CurrentUser};
use crate::error::ApiError;
use crate::links::can_modify;
use crate::rules::is_crawler_tag;
use anyhow::Context;
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use axum_extra::extract::Query;
use chrono::{Datelike, Months, NaiveDate, NaiveDateTime, TimeDelta};
use link_shortener_store::Store;
use quick_cache::sync::Cache;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

const CACHE_TTL: Duration = Duration::from_secs(5 * 60);
const CACHE_CAPACITY: usize = 1024;
const BREAKDOWN_ROWS: u32 = 10;

/// Filter value for visits PostHog has no value for
const UNKNOWN: &str = "unknown";

/// PostHog query (HogQL) for an event's crawler tags, empty for people
const CRAWLER_TAGS: &str = "JSONExtract(ifNull(properties.crawler_tags, '[]'), 'Array(String)')";

/// How far back a link's visits are counted
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Deserialize, utoipa::ToSchema)]
pub enum Range {
    #[serde(rename = "7d")]
    Last7Days,
    #[default]
    #[serde(rename = "30d")]
    Last30Days,
    #[serde(rename = "90d")]
    Last90Days,
    #[serde(rename = "all")]
    AllTime,
}

impl Range {
    /// Days covered ending today, or `None` for all time
    const fn days(self) -> Option<u32> {
        match self {
            Self::Last7Days => Some(7),
            Self::Last30Days => Some(30),
            Self::Last90Days => Some(90),
            Self::AllTime => None,
        }
    }

    const fn bucket(self) -> Bucket {
        match self {
            Self::Last7Days | Self::Last30Days => Bucket::Day,
            Self::Last90Days => Bucket::Week,
            Self::AllTime => Bucket::Month,
        }
    }
}

/// The span of time each bar covers
#[derive(Clone, Copy, Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Bucket {
    Day,
    /// Starting on Sunday, as PostHog's `toStartOfWeek` does
    Week,
    Month,
}

impl Bucket {
    const fn hogql(self) -> &'static str {
        match self {
            Self::Day => "toDate(timestamp)",
            Self::Week => "toStartOfWeek(toDate(timestamp))",
            Self::Month => "toStartOfMonth(toDate(timestamp))",
        }
    }

    fn start_of(self, date: NaiveDate) -> NaiveDate {
        match self {
            Self::Day => date,
            Self::Week => date - TimeDelta::days(date.weekday().num_days_from_sunday().into()),
            Self::Month => date.with_day(1).unwrap_or(date),
        }
    }

    fn next(self, start: NaiveDate) -> NaiveDate {
        match self {
            Self::Day => start + TimeDelta::days(1),
            Self::Week => start + TimeDelta::days(7),
            Self::Month => start + Months::new(1),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Dimension {
    Traffic,
    Source,
    DeviceType,
    Os,
    Browser,
    Country,
    City,
}

impl Dimension {
    const ALL: [Self; 7] = [
        Self::Traffic,
        Self::Source,
        Self::DeviceType,
        Self::Os,
        Self::Browser,
        Self::Country,
        Self::City,
    ];

    const fn name(self) -> &'static str {
        match self {
            Self::Traffic => "traffic",
            Self::Source => "source",
            Self::DeviceType => "device_type",
            Self::Os => "os",
            Self::Browser => "browser",
            Self::Country => "country",
            Self::City => "city",
        }
    }

    /// PostHog query (HogQL) for the value an event is counted under
    fn hogql(self) -> String {
        match self {
            Self::Traffic => format!(
                "arrayJoin(if(empty({CRAWLER_TAGS}), ['people'], arrayConcat(['bots'], {CRAWLER_TAGS})))"
            ),
            Self::Source => "properties.utm_source".into(),
            Self::DeviceType => "properties.$device_type".into(),
            // the server's OS, on events from before PostHog parsed user agents
            Self::Os => "nullIf(properties.$os, 'NixOS')".into(),
            // PostHog reports most in-app browsers as their host browser
            Self::Browser => "coalesce(properties.in_app_browser, properties.$browser)".into(),
            Self::Country => "properties.$geoip_country_name".into(),
            Self::City => "properties.$geoip_city_name".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(try_from = "String")]
enum Traffic {
    All,
    People,
    Bots,
    Tag(String),
}

impl TryFrom<String> for Traffic {
    type Error = String;

    fn try_from(value: String) -> Result<Self, String> {
        match value.as_str() {
            "all" => Ok(Self::All),
            "people" => Ok(Self::People),
            "bots" => Ok(Self::Bots),
            tag if is_crawler_tag(tag) => Ok(Self::Tag(value)),
            _ => Err(format!("unknown traffic {value:?}")),
        }
    }
}

#[derive(Clone, Serialize, utoipa::ToSchema)]
pub struct LinkStats {
    visits: u64,
    visitors: u64,
    bucket: Bucket,
    /// Oldest first, ending with the bar that holds today
    bars: Vec<Bar>,
    breakdowns: Breakdowns,
}

#[derive(Clone, Serialize, utoipa::ToSchema)]
pub struct Bar {
    start: NaiveDate,
    visits: u64,
}

/// Visits per value, most first, each ignoring its own filter
#[derive(Clone, Default, Serialize, utoipa::ToSchema)]
pub struct Breakdowns {
    /// `people`, `bots` and each crawler tag
    traffic: Vec<Count>,
    /// `utm_source`, such as `qr` for scans of the link's QR code
    source: Vec<Count>,
    device_type: Vec<Count>,
    os: Vec<Count>,
    browser: Vec<Count>,
    country: Vec<Count>,
    city: Vec<Count>,
}

impl Breakdowns {
    const fn get_mut(&mut self, dimension: Dimension) -> &mut Vec<Count> {
        match dimension {
            Dimension::Traffic => &mut self.traffic,
            Dimension::Source => &mut self.source,
            Dimension::DeviceType => &mut self.device_type,
            Dimension::Os => &mut self.os,
            Dimension::Browser => &mut self.browser,
            Dimension::Country => &mut self.country,
            Dimension::City => &mut self.city,
        }
    }
}

#[derive(Clone, Serialize, utoipa::ToSchema)]
pub struct Count {
    /// `None` when PostHog doesn't know it
    value: Option<String>,
    visits: u64,
}

#[derive(Deserialize)]
struct QueryResponse {
    /// Kind, key, visits, distinct visitors and today's date in the project's time zone
    results: Vec<(String, Option<String>, u64, u64, NaiveDate)>,
}

#[derive(Clone, PartialEq, Eq, Hash, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct StatsParams {
    #[serde(default)]
    #[param(inline)]
    range: Range,
    /// Any of `people` (the default), `bots` and crawler tags, or `all`
    #[serde(default)]
    #[param(value_type = Option<Vec<String>>)]
    traffic: Vec<Traffic>,
    /// Any of the values from the breakdowns, with `unknown` for visits without one
    #[serde(default)]
    source: Vec<String>,
    #[serde(default)]
    device_type: Vec<String>,
    #[serde(default)]
    os: Vec<String>,
    #[serde(default)]
    browser: Vec<String>,
    #[serde(default)]
    country: Vec<String>,
    #[serde(default)]
    city: Vec<String>,
}

impl StatsParams {
    fn values(&self, dimension: Dimension) -> &[String] {
        match dimension {
            Dimension::Traffic => &[],
            Dimension::Source => &self.source,
            Dimension::DeviceType => &self.device_type,
            Dimension::Os => &self.os,
            Dimension::Browser => &self.browser,
            Dimension::Country => &self.country,
            Dimension::City => &self.city,
        }
    }

    /// PostHog query (HogQL) condition for this dimension's filter, with a `{name_index}` placeholder per value
    fn condition(&self, dimension: Dimension) -> Option<String> {
        let parts: Vec<String> = if dimension == Dimension::Traffic {
            if self.traffic.contains(&Traffic::All) {
                return None;
            }
            if self.traffic.is_empty() {
                return Some(format!("empty({CRAWLER_TAGS})"));
            }

            self.traffic
                .iter()
                .enumerate()
                .map(|(index, traffic)| match traffic {
                    Traffic::All | Traffic::People => format!("empty({CRAWLER_TAGS})"),
                    Traffic::Bots => format!("notEmpty({CRAWLER_TAGS})"),
                    Traffic::Tag(_) => format!("has({CRAWLER_TAGS}, {{traffic_{index}}})"),
                })
                .collect()
        } else {
            self.values(dimension)
                .iter()
                .enumerate()
                .map(|(index, value)| match value.as_str() {
                    UNKNOWN => format!("isNull({})", dimension.hogql()),
                    _ => format!("{} = {{{}_{index}}}", dimension.hogql(), dimension.name()),
                })
                .collect()
        };

        (!parts.is_empty()).then(|| format!("({})", parts.join(" OR ")))
    }

    /// PostHog query (HogQL) for visits per bucket, the totals, and visits per value of each dimension
    fn hogql(&self) -> String {
        let since = match self.range.days() {
            Some(_) => "timestamp >= toStartOfDay(now()) - toIntervalDay({days} - 1)",
            None => "timestamp >= {created_at}",
        };
        let filter = |skip: Option<Dimension>| {
            let mut filter =
                format!("event = 'link_visited' AND properties.link_id = {{link_id}} AND {since}");
            for dimension in Dimension::ALL.into_iter().filter(|d| Some(*d) != skip) {
                if let Some(condition) = self.condition(dimension) {
                    filter.push_str(" AND ");
                    filter.push_str(&condition);
                }
            }
            filter
        };

        let all = filter(None);
        let bucket = self.range.bucket().hogql();
        let mut parts = vec![
            format!(
                "SELECT 'bar', toString({bucket}) AS key, count(), uniq(distinct_id), today() FROM events WHERE {all} GROUP BY key"
            ),
            format!(
                "SELECT 'total', NULL, count(), uniq(distinct_id), today() FROM events WHERE {all}"
            ),
        ];
        parts.extend(Dimension::ALL.map(|dimension| {
            format!(
                "SELECT '{}', {} AS key, count(), uniq(distinct_id), today() FROM events WHERE {} \
                 GROUP BY key ORDER BY count() DESC, key LIMIT {BREAKDOWN_ROWS}",
                dimension.name(),
                dimension.hogql(),
                filter(Some(dimension)),
            )
        }));

        parts.join(" UNION ALL ")
    }
}

struct Client {
    http: reqwest::Client,
    url: String,
    api_key: String,
}

/// Reads visit counts back from PostHog
pub struct Stats {
    client: Option<Client>,
    cache: Cache<(Uuid, StatsParams), (Instant, LinkStats)>,
}

impl Stats {
    pub fn new(api_key: Option<String>, project_id: Option<String>) -> Self {
        let non_empty = |value: Option<String>| value.filter(|value| !value.is_empty());
        let client = non_empty(api_key)
            .zip(non_empty(project_id))
            .map(|(api_key, project_id)| Client {
                http: reqwest::Client::new(),
                url: format!("https://us.posthog.com/api/projects/{project_id}/query/"),
                api_key,
            });

        Self {
            client,
            cache: Cache::new(CACHE_CAPACITY),
        }
    }

    async fn link(
        &self,
        id: Uuid,
        created_at: NaiveDateTime,
        params: StatsParams,
    ) -> Result<LinkStats, ApiError> {
        let key = (id, params);
        if let Some((fetched, stats)) = self.cache.get(&key)
            && fetched.elapsed() < CACHE_TTL
        {
            return Ok(stats);
        }

        let client = self.client.as_ref().ok_or(ApiError::Unavailable(
            "visit stats are not configured".into(),
        ))?;
        let stats = client
            .link(id, created_at, &key.1)
            .await
            .map_err(ApiError::Internal)?;

        self.cache.insert(key, (Instant::now(), stats.clone()));
        Ok(stats)
    }
}

impl Client {
    async fn link(
        &self,
        id: Uuid,
        created_at: NaiveDateTime,
        params: &StatsParams,
    ) -> anyhow::Result<LinkStats> {
        let mut values = json!({ "link_id": id });
        match params.range.days() {
            Some(days) => values["days"] = Value::from(days),
            None => values["created_at"] = Value::from(created_at.to_string()),
        }
        for (index, traffic) in params.traffic.iter().enumerate() {
            if let Traffic::Tag(tag) = traffic {
                values[format!("traffic_{index}")] = Value::from(tag.as_str());
            }
        }
        for dimension in Dimension::ALL {
            for (index, value) in params.values(dimension).iter().enumerate() {
                if value != UNKNOWN {
                    values[format!("{}_{index}", dimension.name())] = Value::from(value.as_str());
                }
            }
        }

        let body = json!({
            "query": { "kind": "HogQLQuery", "query": params.hogql(), "values": values },
            "name": "link stats",
        });

        let response: QueryResponse = self
            .http
            .post(&self.url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .context("PostHog query request failed")?
            .error_for_status()
            .context("PostHog rejected the query")?
            .json()
            .await
            .context("PostHog query returned an unexpected body")?;

        let today = response
            .results
            .first()
            .map(|row| row.4)
            .context("PostHog returned no rows")?;
        let bucket = params.range.bucket();
        let first_day = match params.range.days() {
            Some(days) => today - TimeDelta::days(i64::from(days) - 1),
            None => created_at.date(),
        };

        let mut stats = LinkStats {
            visits: 0,
            visitors: 0,
            bucket,
            bars: Vec::new(),
            breakdowns: Breakdowns::default(),
        };

        let mut start = bucket.start_of(first_day);
        while start <= today {
            stats.bars.push(Bar { start, visits: 0 });
            start = bucket.next(start);
        }

        for (kind, key, visits, visitors, _) in response.results {
            match kind.as_str() {
                "bar" => {
                    let start = key.and_then(|key| key.parse::<NaiveDate>().ok());
                    if let Some(bar) = stats.bars.iter_mut().find(|bar| Some(bar.start) == start) {
                        bar.visits = visits;
                    }
                }
                "total" => {
                    stats.visits = visits;
                    stats.visitors = visitors;
                }
                kind => {
                    if let Some(dimension) = Dimension::ALL.into_iter().find(|d| d.name() == kind) {
                        stats
                            .breakdowns
                            .get_mut(dimension)
                            .push(Count { value: key, visits });
                    }
                }
            }
        }

        for dimension in Dimension::ALL {
            stats
                .breakdowns
                .get_mut(dimension)
                .sort_by(|a, b| b.visits.cmp(&a.visits).then_with(|| a.value.cmp(&b.value)));
        }

        Ok(stats)
    }
}

/// Returns a link's visits over a range of time
#[utoipa::path(
    get,
    path = "/api/links/{id}/stats",
    tag = "links",
    params(StatsParams),
    responses(
        (status = OK, body = LinkStats),
        (status = SERVICE_UNAVAILABLE, description = "Visit stats are not configured"),
    )
)]
pub async fn link_stats(
    user: CurrentUser,
    Extension(auth): Extension<Arc<AuthConfig>>,
    Extension(stats): Extension<Arc<Stats>>,
    State(store): State<Arc<Store>>,
    Path(id): Path<Uuid>,
    Query(params): Query<StatsParams>,
) -> Result<Json<LinkStats>, ApiError> {
    let link = store
        .links()
        .find_by_id(id)
        .await?
        .ok_or(ApiError::NotFound)?;

    if !can_modify(&user, &auth, &link) {
        return Err(ApiError::Forbidden);
    }

    Ok(Json(stats.link(id, link.created_at, params).await?))
}
