//! `autoversion.rule`: the rule itself, reading the current version,
//! classifying the change between two surfaces and advancing the version.
//! Each cause is one refusal kind of docs/FIXTURES.md, under the name
//! `RuleError::refusal` gives it.

use autoversion::RuleError;
use wisent_errors::{Cause, Operation};

use super::{DOCS, PLAIN};

pub static NOT_CANONICAL: Cause = Cause {
    id: "not-canonical",
    failed: "reading the current version",
    cause: "the version is not a canonical coordinate: empty, or holding whitespace or a character other than alphanumerics, '.', '_' and '-'",
    remedy: "pass --current as the version exactly as published, such as 1.2.3, with no surrounding whitespace or build metadata",
    test: "tests/rule/main.rs::a_version_it_cannot_read_as_numbers_is_refused_naming_the_slot",
    ..PLAIN
};

pub static NOT_A_TRIPLE: Cause = Cause {
    id: "not-a-triple",
    failed: "reading the current version as major.minor.patch",
    cause: "the version does not have exactly three slots, so there is no slot to advance",
    remedy: "name the next version explicitly, or pass --current as a major.minor.patch version",
    test: "tests/rule/main.rs::a_version_it_cannot_read_as_numbers_is_refused_naming_the_slot",
    ..PLAIN
};

pub static NOT_NUMERIC: Cause = Cause {
    id: "not-numeric",
    failed: "reading each slot of the current version as a number",
    cause: "a slot is not all digits or holds no unsigned 128-bit number, and advancing it would invent an ordering",
    remedy: "name the next version explicitly, or pass --current as a version whose three slots are numbers",
    test: "tests/rule/main.rs::a_version_it_cannot_read_as_numbers_is_refused_naming_the_slot",
    ..PLAIN
};

pub static EMPTY_SURFACE: Cause = Cause {
    id: "empty-surface",
    failed: "reading the published and candidate surfaces",
    cause: "a surface holds no name, which is far more likely a broken extractor than a product that promises nothing",
    remedy: "correct the surface extraction so it lists the product's public names, then ask again",
    test: "tests/rule/main.rs::an_empty_candidate_surface_is_refused_and_json_may_follow_the_command",
    ..PLAIN
};

pub static SLOT_EXHAUSTED: Cause = Cause {
    id: "slot-exhausted",
    failed: "advancing the current version",
    cause: "the slot the change advances already holds the largest number a slot holds, and wrapping it would make a smaller next version",
    remedy: "name the next version explicitly",
    test: "tests/rule/main.rs::a_slot_at_the_largest_number_is_refused_rather_than_wrapped",
    ..PLAIN
};

pub static RULE: Operation = Operation {
    id: "autoversion.rule",
    entries: &[],
    uses: &[],
    summary: "read the current version, classify the change between the published and candidate surfaces, and advance the slot docs/SPEC.md names",
    docs: DOCS,
    causes: &[&NOT_CANONICAL, &NOT_A_TRIPLE, &NOT_NUMERIC, &EMPTY_SURFACE, &SLOT_EXHAUSTED],
    warnings: &[],
};

/// The cause the rule's refusal `error` is.
pub fn cause(error: &RuleError) -> &'static Cause {
    match error {
        RuleError::NotCanonical(_) => &NOT_CANONICAL,
        RuleError::NotATriple(_) => &NOT_A_TRIPLE,
        RuleError::NotNumeric(_) => &NOT_NUMERIC,
        RuleError::EmptySurface(_) => &EMPTY_SURFACE,
        RuleError::SlotExhausted(_) => &SLOT_EXHAUSTED,
    }
}
