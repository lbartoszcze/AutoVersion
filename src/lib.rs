//! AutoVersion: one versioning rule, several implementations, one contract.
//!
//! The rule answers a single question: given the version currently published,
//! the public surface that was published, and the surface being proposed — what
//! kind of change is this, and what is the next version? See `docs/SPEC.md`;
//! conformance is `docs/FIXTURES.md`.
//!
//! It holds no knowledge of release channels, credentials, build systems, or any
//! product's layout. Surface extraction belongs to the product.

pub mod surfaces;

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fmt;

use serde_json::{json, Value};

pub const BREAKING: &str = "breaking";
pub const ADDITIVE: &str = "additive";
pub const INTERNAL: &str = "internal";

/// A refusal. [`RuleError::refusal`] is the stable name the fixtures use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleError {
    NotCanonical(String),
    NotATriple(String),
    NotNumeric(String),
    EmptySurface(String),
}

impl RuleError {
    pub fn refusal(&self) -> &'static str {
        match self {
            RuleError::NotCanonical(_) => "not-canonical",
            RuleError::NotATriple(_) => "not-a-triple",
            RuleError::NotNumeric(_) => "not-numeric",
            RuleError::EmptySurface(_) => "empty-surface",
        }
    }
}

impl fmt::Display for RuleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuleError::NotCanonical(message)
            | RuleError::NotATriple(message)
            | RuleError::NotNumeric(message)
            | RuleError::EmptySurface(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for RuleError {}

/// A segment that survives a URL path and a filesystem key unchanged.
pub fn is_canonical(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version {
    pub major: u128,
    pub minor: u128,
    pub patch: u128,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Version {
    /// While the major slot is zero, the minor slot carries compatibility.
    pub fn is_unstable(&self) -> bool {
        self.major == 0
    }

    pub fn parse(value: &str) -> Result<Version, RuleError> {
        if !is_canonical(value) {
            return Err(RuleError::NotCanonical(format!(
                "{value:?} is not a canonical coordinate: expected a non-empty segment of \
                 alphanumerics, '.', '_' and '-', with no surrounding whitespace"
            )));
        }
        let slots: Vec<&str> = value.split('.').collect();
        let [major, minor, patch] = slots.as_slice() else {
            return Err(RuleError::NotATriple(format!(
                "{value:?} is not a major.minor.patch triple, so there is no slot to advance; \
                 name the next version explicitly"
            )));
        };
        let numeric = |slot: &str| {
            (!slot.is_empty() && slot.bytes().all(|byte| byte.is_ascii_digit()))
                .then(|| slot.parse::<u128>().ok())
                .flatten()
        };
        match (numeric(major), numeric(minor), numeric(patch)) {
            (Some(major), Some(minor), Some(patch)) => Ok(Version {
                major,
                minor,
                patch,
            }),
            _ => Err(RuleError::NotNumeric(format!(
                "{value:?} has a non-numeric slot, so advancing it would invent an ordering; \
                 name the next version explicitly"
            ))),
        }
    }

    /// The version this change produces. See the table in `docs/SPEC.md`.
    pub fn advance(&self, change: &str) -> Version {
        let Version {
            major,
            minor,
            patch,
        } = *self;
        if change == BREAKING {
            if self.is_unstable() {
                return Version {
                    major,
                    minor: minor + 1,
                    patch: 0,
                };
            }
            return Version {
                major: major + 1,
                minor: 0,
                patch: 0,
            };
        }
        if change == ADDITIVE && !self.is_unstable() {
            return Version {
                major,
                minor: minor + 1,
                patch: 0,
            };
        }
        // An additive change under an unstable major is compatible, exactly like
        // an internal one, and the patch slot is the only one left to hold it.
        Version {
            major,
            minor,
            patch: patch + 1,
        }
    }
}

fn surface<'a>(names: &'a [String], side: &str) -> Result<BTreeSet<&'a str>, RuleError> {
    let collected: BTreeSet<&str> = names.iter().map(String::as_str).collect();
    if collected.is_empty() {
        return Err(RuleError::EmptySurface(format!(
            "the {side} surface is empty, which is far more likely to be a broken extractor \
             than a product that promises nothing"
        )));
    }
    Ok(collected)
}

/// `(change, removed, added)`. A declaration may only escalate.
pub fn classify(
    published: &[String],
    candidate: &[String],
    declared_breaking: bool,
) -> Result<(&'static str, Vec<String>, Vec<String>), RuleError> {
    let before = surface(published, "published")?;
    let after = surface(candidate, "candidate")?;
    let removed: Vec<String> = before
        .difference(&after)
        .map(|name| name.to_string())
        .collect();
    let added: Vec<String> = after
        .difference(&before)
        .map(|name| name.to_string())
        .collect();
    let change = if declared_breaking || !removed.is_empty() {
        BREAKING
    } else if !added.is_empty() {
        ADDITIVE
    } else {
        INTERNAL
    };
    Ok((change, removed, added))
}

/// The whole answer: what changed, and what to call it.
pub fn decide(
    current: &str,
    published: &[String],
    candidate: &[String],
    declared_breaking: bool,
) -> Result<Value, RuleError> {
    let version = Version::parse(current)?;
    let (change, removed, added) = classify(published, candidate, declared_breaking)?;
    Ok(json!({
        "current": version.to_string(),
        "change": change,
        "next": version.advance(change).to_string(),
        "removed": removed,
        "added": added,
    }))
}

/// One token of a coordinate: a number sorts before any text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// Decimal digits without leading zeros (`"0"` for zero), so numbers of any
    /// size compare without overflow.
    Number(String),
    Text(String),
}

impl Ord for Token {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Token::Number(left), Token::Number(right)) => {
                left.len().cmp(&right.len()).then_with(|| left.cmp(right))
            }
            (Token::Number(_), Token::Text(_)) => Ordering::Less,
            (Token::Text(_), Token::Number(_)) => Ordering::Greater,
            (Token::Text(left), Token::Text(right)) => left.cmp(right),
        }
    }
}

impl PartialOrd for Token {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The digits of a token read as an integer the way the rule's first port
/// read it: surrounding whitespace and a leading `+` allowed, underscores only
/// between digits.
fn integer(token: &str) -> Option<String> {
    let token = token.trim();
    let token = token.strip_prefix('+').unwrap_or(token);
    let bytes = token.as_bytes();
    let well_formed = !bytes.is_empty()
        && bytes.first().is_some_and(u8::is_ascii_digit)
        && bytes.last().is_some_and(u8::is_ascii_digit)
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_digit() || *byte == b'_')
        && !token.contains("__");
    if !well_formed {
        return None;
    }
    let digits: String = token.chars().filter(char::is_ascii_digit).collect();
    let significant = digits.trim_start_matches('0');
    Some(if significant.is_empty() {
        "0".to_string()
    } else {
        significant.to_string()
    })
}

/// Split on `.` and `-`; numeric tokens as numbers, sorting before strings.
/// Ordering works on coordinates that are not triples, because a version that
/// can never be advanced can still be compared.
pub fn version_tuple(value: &str) -> Vec<Token> {
    value
        .split(['.', '-'])
        .map(|token| match integer(token) {
            Some(number) => Token::Number(number),
            None => Token::Text(token.to_string()),
        })
        .collect()
}

/// True when `candidate` is strictly newer than `installed`.
pub fn version_newer(installed: &str, candidate: &str) -> bool {
    version_tuple(installed) < version_tuple(candidate)
}
