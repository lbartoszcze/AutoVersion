//! `autoversion.decide`, `autoversion.check` and `autoversion.surface`: the
//! commands, each made of the layers it reads through; `check` alone refuses
//! on a step of its own, the version the product declares.

use wisent_errors::{Cause, Operation};

use super::{DOCS, DOCUMENT, PLAIN, PYTHON, RULE};

pub static DECIDE: Operation = Operation {
    id: "autoversion.decide",
    entries: &["autoversion decide"],
    uses: &[&DOCUMENT, &RULE],
    summary: "classify the change between two surface documents and name the next version",
    docs: DOCS,
    causes: &[],
    warnings: &[],
};

pub static EXPECTED_VERSION_UNMADE: Cause = Cause {
    id: "expected-version-unmade",
    failed: "holding the version the product declares with --expect to the decision",
    cause: "the surface change requires another next version than the one the product declares; the decision printed on stdout names the version it requires",
    remedy: "declare the version the decision names as next, or change the candidate package's public names (or pass --breaking) so the change makes the declared one",
    test: "tests/cli/main.rs::check_reads_two_python_packages_and_refuses_a_declared_version_the_change_does_not_make",
    ..PLAIN
};

pub static CHECK: Operation = Operation {
    id: "autoversion.check",
    entries: &["autoversion check"],
    uses: &[&PYTHON, &RULE],
    summary: "read two Python packages' public names, classify the change, name the next version and hold the version the product declares to it",
    docs: DOCS,
    causes: &[&EXPECTED_VERSION_UNMADE],
    warnings: &[],
};

pub static SURFACE: Operation = Operation {
    id: "autoversion.surface",
    entries: &["autoversion surface"],
    uses: &[&PYTHON],
    summary: "print a Python package's public names: its __all__, or with --module-bindings its public module-level bindings",
    docs: DOCS,
    causes: &[],
    warnings: &[],
};
