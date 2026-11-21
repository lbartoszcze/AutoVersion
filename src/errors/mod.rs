//! Every way `autoversion` refuses a caller, declared as
//! `wisent_errors::Operation`s, one per functionality: the product itself
//! (`autoversion`: the command line it reads), the commands that refuse on
//! their own (`autoversion.decide`, `autoversion.check`,
//! `autoversion.surface`), and the layers they run: the surface document
//! `decide` reads (`autoversion.document`), the Python surface reader
//! (`autoversion.python`) and the rule itself (`autoversion.rule`). Each
//! cause says what failed, why, the state it shows (the sentence the caller
//! has always read), the fix and the real test that provokes it; every
//! refusal is raised from one of them through [`refuse`], so the sentence a
//! caller reads and the envelope a program reads say the same thing.
//! `check` holds the catalogue to the standard.

// Read by its own test alone: the test holds the catalogue to the standard.
#[cfg(test)]
mod check;
pub mod commands;
pub mod document;
pub mod product;
pub mod python;
pub mod rule;

use std::fmt;

use wisent_errors::{Cause, Code, Failure, Operation};

pub use commands::{CHECK, DECIDE, SURFACE};
pub use document::DOCUMENT;
pub use product::AUTOVERSION;
pub use python::PYTHON;
pub use rule::RULE;

/// The service every envelope names.
pub const SERVICE: &str = "autoversion";

/// The status a refusal that is not usage ends with, as the CLI always
/// ended: the rule declined to invent an answer.
pub const FAILURE_EXIT: i32 = 1; // https://www.gnu.org/software/bash/manual/html_node/Exit-Status.html

/// The page every cause is a line on: the README's refusals.
pub(crate) const DOCS: &str = "https://github.com/lbartoszcze/AutoVersion/blob/main/README.md#refusals";

/// Every cause starts from this one and names what is its own.
pub(crate) const PLAIN: Cause = Cause {
    id: "",
    code: Code::Refused,
    usage: false,
    exit: None,
    failed: "",
    cause: "",
    observed: "{refusal}",
    remedy: "",
    test: "",
};

/// The functionalities `autoversion` declares: the product, its commands,
/// then the layers they use. Only the catalogue check reads the list; every
/// refusal names its own functionality.
#[cfg(test)]
pub static OPERATIONS: &[&Operation] = &[&AUTOVERSION, &DECIDE, &CHECK, &SURFACE, &DOCUMENT, &PYTHON, &RULE];

/// What the caller loses when `operation` fails.
fn impact(operation: &Operation) -> &'static str {
    match operation.id {
        id if id == AUTOVERSION.id => "the command the caller ran: it ran nothing and printed no answer",
        id if id == CHECK.id => "the release the product declared: its declared version is not the one the surface change makes",
        id if id == DOCUMENT.id => "the decision decide would have made: a surface it compares could not be read",
        id if id == PYTHON.id => "the decision or surface the command would have answered: a package's public names could not be read",
        id if id == RULE.id => "the version decision: the rule declined to invent a next version",
        // A command whose own steps hold no cause: what is lost is its answer.
        _ => "the answer of the command the caller ran",
    }
}

/// A refusal one declared cause raised: the functionality, the cause, and
/// the sentence the caller has always read.
#[derive(Debug)]
pub struct Declared {
    operation: &'static Operation,
    cause: &'static Cause,
    refusal: String,
}

impl Declared {
    /// Whether the invocation itself was refused, which the shell answers
    /// with the usage.
    pub fn usage(&self) -> bool {
        self.cause.usage
    }

    /// The status a shell command ends with on this refusal.
    pub fn exit_code(&self) -> i32 {
        self.cause.exit_code(FAILURE_EXIT)
    }

    /// The envelope, its observed state the refusal as the caller read it.
    pub fn failure(&self) -> Failure {
        self.cause
            .failure(self.operation.id, SERVICE, &[("refusal", self.refusal.as_str())])
            .impact(impact(self.operation))
    }
}

impl fmt::Display for Declared {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.refusal)
    }
}

impl std::error::Error for Declared {}

/// `cause` of `operation`, the caller reading `refusal`.
pub fn refuse(operation: &'static Operation, cause: &'static Cause, refusal: impl Into<String>) -> Declared {
    Declared {
        operation,
        cause,
        refusal: refusal.into(),
    }
}

impl From<autoversion::RuleError> for Declared {
    /// The rule's refusal, as the cause of its kind.
    fn from(error: autoversion::RuleError) -> Self {
        refuse(&RULE, rule::cause(&error), error.to_string())
    }
}

impl From<autoversion::surfaces::SurfaceError> for Declared {
    /// The Python surface reader's refusal, as the cause of its kind.
    fn from(error: autoversion::surfaces::SurfaceError) -> Self {
        refuse(&PYTHON, python::cause(error.refusal), error.message)
    }
}
