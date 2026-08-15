use entity::{link_rules, links};
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};

const REGEX_SIZE_LIMIT: usize = 64 * 1024;

pub const MAX_PATTERN_LEN: usize = 512;

pub const MAX_RULES: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MatchKind {
    Platform,
    Regex,
}

impl MatchKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Platform => "platform",
            Self::Regex => "regex",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "platform" => Some(Self::Platform),
            "regex" => Some(Self::Regex),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    Ios,
    Android,
    Mobile,
    Desktop,
    Windows,
    Macos,
    Linux,
    Chromeos,
    Bot,
}

impl Platform {
    pub const ALL: [Self; 9] = [
        Self::Ios,
        Self::Android,
        Self::Mobile,
        Self::Desktop,
        Self::Windows,
        Self::Macos,
        Self::Linux,
        Self::Chromeos,
        Self::Bot,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ios => "ios",
            Self::Android => "android",
            Self::Mobile => "mobile",
            Self::Desktop => "desktop",
            Self::Windows => "windows",
            Self::Macos => "macos",
            Self::Linux => "linux",
            Self::Chromeos => "chromeos",
            Self::Bot => "bot",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|platform| platform.as_str().eq_ignore_ascii_case(value))
    }

    pub fn matches(self, user_agent: &str) -> bool {
        match self {
            Self::Ios => ["iphone", "ipad", "ipod"]
                .iter()
                .any(|marker| contains_ci(user_agent, marker)),
            Self::Android => contains_ci(user_agent, "android"),
            Self::Chromeos => contains_ci(user_agent, "cros"),
            Self::Windows => contains_ci(user_agent, "windows"),
            Self::Macos => {
                !Self::Ios.matches(user_agent)
                    && (contains_ci(user_agent, "macintosh") || contains_ci(user_agent, "mac os x"))
            }
            Self::Linux => {
                contains_ci(user_agent, "linux")
                    && !Self::Android.matches(user_agent)
                    && !Self::Chromeos.matches(user_agent)
            }
            Self::Mobile => {
                contains_ci(user_agent, "mobi")
                    || Self::Android.matches(user_agent)
                    || Self::Ios.matches(user_agent)
            }
            Self::Desktop => {
                !Self::Mobile.matches(user_agent)
                    && [Self::Windows, Self::Macos, Self::Linux, Self::Chromeos]
                        .into_iter()
                        .any(|platform| platform.matches(user_agent))
            }
            Self::Bot => [
                "bot",
                "crawler",
                "spider",
                "slurp",
                "facebookexternalhit",
                "whatsapp",
                "headlesschrome",
                "curl/",
                "wget/",
                "python-requests",
                "go-http-client",
            ]
            .iter()
            .any(|marker| contains_ci(user_agent, marker)),
        }
    }
}

fn contains_ci(haystack: &str, needle: &str) -> bool {
    debug_assert!(needle.bytes().all(|b| !b.is_ascii_uppercase()));
    let (haystack, needle) = (haystack.as_bytes(), needle.as_bytes());
    needle.len() <= haystack.len()
        && haystack
            .windows(needle.len())
            .any(|window| window.eq_ignore_ascii_case(needle))
}

fn compile_regex(pattern: &str) -> Result<Regex, regex::Error> {
    RegexBuilder::new(pattern)
        .case_insensitive(true)
        .size_limit(REGEX_SIZE_LIMIT)
        .build()
}

pub fn validate_pattern(kind: MatchKind, pattern: &str) -> Result<(), String> {
    if pattern.is_empty() {
        return Err("pattern must not be empty".into());
    }
    if pattern.len() > MAX_PATTERN_LEN {
        return Err(format!("pattern must be at most {MAX_PATTERN_LEN} bytes",));
    }
    match kind {
        MatchKind::Platform => Platform::parse(pattern).map(|_| ()).ok_or_else(|| {
            let known = Platform::ALL.map(Platform::as_str).join(", ");
            format!("unknown platform {pattern:?}, expected one of: {known}")
        }),
        MatchKind::Regex => compile_regex(pattern)
            .map(|_| ())
            .map_err(|e| format!("invalid regex: {e}")),
    }
}

fn rule_matches(rule: &link_rules::Model, user_agent: &str) -> bool {
    match MatchKind::parse(&rule.kind) {
        Some(MatchKind::Platform) => {
            Platform::parse(&rule.pattern).is_some_and(|platform| platform.matches(user_agent))
        }
        Some(MatchKind::Regex) => {
            compile_regex(&rule.pattern).is_ok_and(|regex| regex.is_match(user_agent))
        }
        None => false,
    }
}

pub fn resolve_target<'a>(
    link: &'a links::Model,
    rules: &'a [link_rules::Model],
    user_agent: Option<&str>,
) -> &'a str {
    let Some(user_agent) = user_agent else {
        return &link.target_url;
    };
    rules
        .iter()
        .find(|rule| rule_matches(rule, user_agent))
        .map_or(link.target_url.as_str(), |rule| rule.target_url.as_str())
}