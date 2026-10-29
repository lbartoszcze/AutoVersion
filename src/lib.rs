//! AutoVersion: one versioning rule, several implementations, one contract.
//!
//! The rule answers a single question: given the version currently published,
//! the public surface that was published, and the surface being proposed — what
//! kind of change is this, and what is the next version? See `docs/SPEC.md`;
//! conformance is `docs/FIXTURES.md`.
//!
//! It holds no knowledge of release channels, credentials, build systems, or any
//! product's layout. Surface extraction belongs to the product.

#[cfg(feature = "surfaces")]
pub mod surfaces;

mod order;
pub use order::{version_newer, version_tuple, Token};

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
    SlotExhausted(String),
}

impl RuleError {
    pub fn refusal(&self) -> &'static str {
        match self {
            RuleError::NotCanonical(_) => "not-canonical",
            RuleError::NotATriple(_) => "not-a-triple",
            RuleError::NotNumeric(_) => "not-numeric",
            RuleError::EmptySurface(_) => "empty-surface",
            RuleError::SlotExhausted(_) => "slot-exhausted",
        }
    }
}

impl fmt::Display for RuleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuleError::NotCanonical(message)
            | RuleError::NotATriple(message)
            | RuleError::NotNumeric(message)
            | RuleError::EmptySurface(message)
            | RuleError::SlotExhausted(message) => f.write_str(message),
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

// https://semver.org/#spec-item-4 : major version zero is for initial development.
const UNSTABLE_MAJOR: u128 = 0;
// https://semver.org/#spec-item-6 : a released change increments one slot by one.
const STEP: u128 = 1;
// https://semver.org/#spec-item-7 : the slots below an incremented one are reset to 0.
const RESET: u128 = 0;

impl Version {
    /// While the major slot is zero, the minor slot carries compatibility.
    pub fn is_unstable(&self) -> bool {
        self.major == UNSTABLE_MAJOR
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
        // A slot is all ASCII digits and fits the slot width; the refusal names which
        // of the two a slot failed, the integer parser's own error included.
        let numeric = |slot: &str| -> Result<u128, String> {
            if slot.is_empty() || !slot.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(format!("slot {slot:?} is not all digits"));
            }
            slot.parse::<u128>()
                .map_err(|error| format!("slot {slot:?} holds no unsigned 128-bit number: {error}"))
        };
        let refuse = |cause: String| {
            RuleError::NotNumeric(format!(
                "{value:?} is not numeric in every slot ({cause}), so advancing it would invent \
                 an ordering; name the next version explicitly"
            ))
        };
        Ok(Version {
            major: numeric(major).map_err(&refuse)?,
            minor: numeric(minor).map_err(&refuse)?,
            patch: numeric(patch).map_err(&refuse)?,
        })
    }

    /// The version this change produces. See the table in `docs/SPEC.md`.
    /// A slot already at the largest number it holds cannot be incremented,
    /// and is refused rather than wrapped to a smaller next version.
    pub fn advance(&self, change: &str) -> Result<Version, RuleError> {
        let Version {
            major,
            minor,
            patch,
        } = *self;
        let step = |slot: u128, name: &str| {
            slot.checked_add(STEP).ok_or_else(|| {
                RuleError::SlotExhausted(format!(
                    "{self} cannot advance its {name} slot: {slot} is the largest number a slot \
                     holds; name the next version explicitly"
                ))
            })
        };
        if change == BREAKING {
            if self.is_unstable() {
                return Ok(Version {
                    major,
                    minor: step(minor, "minor")?,
                    patch: RESET,
                });
            }
            return Ok(Version {
                major: step(major, "major")?,
                minor: RESET,
                patch: RESET,
            });
        }
        if change == ADDITIVE && !self.is_unstable() {
            return Ok(Version {
                major,
                minor: step(minor, "minor")?,
                patch: RESET,
            });
        }
        // An additive change under an unstable major is compatible, exactly like
        // an internal one, and the patch slot is the only one left to hold it.
        Ok(Version {
            major,
            minor,
            patch: step(patch, "patch")?,
        })
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
        "next": version.advance(change)?.to_string(),
        "removed": removed,
        "added": added,
    }))
}
