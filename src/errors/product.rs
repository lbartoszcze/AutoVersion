//! `autoversion`: what every command shares: the command line read before
//! anything else. Each refusal here is answered with the usage and exits 2.

use wisent_errors::{Cause, Operation};

use super::{CHECK, DECIDE, DOCS, PLAIN, SURFACE};

pub static COMMAND_REFUSED: Cause = Cause {
    id: "command-refused",
    usage: true,
    failed: "reading the command autoversion is asked to run",
    cause: "no command was named, or the word in its place is not decide, check, surface or order",
    remedy: "run `autoversion --help` and name one of the commands it lists: decide, check, surface or order",
    test: "tests/cli/main.rs::a_wrong_invocation_is_refused_with_the_usage",
    ..PLAIN
};

pub static FLAG_UNRECOGNIZED: Cause = Cause {
    id: "flag-unrecognized",
    usage: true,
    failed: "reading the flags after the command",
    cause: "a flag the command does not take was given: each command reads only the flags its own usage line lists",
    remedy: "drop the flag the refusal names, or give only the flags `autoversion --help` lists on the command's own line",
    test: "tests/rule/main.rs::every_required_flag_is_named_when_it_is_missing",
    ..PLAIN
};

pub static FLAG_INCOMPLETE: Cause = Cause {
    id: "flag-incomplete",
    usage: true,
    failed: "reading the values the command's flags hold",
    cause: "a flag the command requires was left out, or a flag that takes a value was the last word, with no value after it",
    remedy: "give the flag the refusal names, with its value, as the command's line in `autoversion --help` shows",
    test: "tests/rule/main.rs::every_required_flag_is_named_when_it_is_missing",
    ..PLAIN
};

pub static AUTOVERSION: Operation = Operation {
    id: "autoversion",
    entries: &["autoversion <command>", "autoversion order"],
    uses: &[&DECIDE, &CHECK, &SURFACE],
    summary: "read the command line, run the command it names (decide, check, surface or order) and print its answer as JSON or as key: value lines",
    docs: DOCS,
    causes: &[&COMMAND_REFUSED, &FLAG_UNRECOGNIZED, &FLAG_INCOMPLETE],
    warnings: &[],
};
