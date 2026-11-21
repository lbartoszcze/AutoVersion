//! The rule's edges through the built `autoversion decide`: every slot that
//! cannot advance (major, minor and patch, stable and under major 0), a slot
//! that is empty or too large for a number, an empty candidate surface, and
//! the answer asked with `--json` after the command. Then every flag a command
//! requires, named when it is missing, and a switch another command does not
//! take. Refusals are whole, with their status: 1 when the rule declined to
//! invent an answer, 2 with the usage when the invocation is wrong.

use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::Output;

const REFUSED: i32 = 1;
const USAGE_STATUS: i32 = 2;
const USAGE: &str = "usage:
  autoversion [--json] decide --current VERSION --published-surface FILE --candidate-surface FILE [--breaking]
  autoversion [--json] check --current VERSION --published-init FILE --candidate-init FILE [--expect VERSION] [--breaking] [--module-bindings]
  autoversion surface --python-init FILE [--module-bindings]
  autoversion [--json] order --older VERSION --newer VERSION";

struct Case {
    root: PathBuf,
}

impl Case {
    fn new(name: &str) -> Self {
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("rule-{name}"));
        // failure: nothing to clear on a first run; a folder an earlier run left that cannot be removed holds only this case's files, each written over.
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn surface(&self, name: &str, names: &[&str]) -> String {
        let path = self.root.join(name);
        std::fs::write(&path, json!({"surface": names}).to_string()).unwrap();
        path.display().to_string()
    }
}

fn autoversion(args: &[&str]) -> Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_autoversion")).args(args).output().expect("the binary runs")
}

/// The status and the refusal of a refused run, which printed nothing.
fn refused(args: &[&str]) -> (Option<i32>, String) {
    let output = autoversion(args);
    assert!(output.stdout.is_empty(), "{args:?} printed {}", String::from_utf8_lossy(&output.stdout));
    (output.status.code(), refusal(&output.stderr).0)
}

/// What a refusal printed on stderr: the sentence it has always printed, and
/// the envelope on the line after it, which names an autoversion
/// functionality, its code, the remedy and the declared cause.
fn refusal(stderr: &[u8]) -> (String, Value) {
    let text = String::from_utf8(stderr.to_vec()).unwrap();
    let Some((sentence, line)) = text.trim_end().rsplit_once('\n') else {
        panic!("no envelope follows the refusal: {text}");
    };
    let envelope: Value = serde_json::from_str(line).unwrap();
    assert!(envelope["failure_point"].as_str().is_some_and(|point| point.starts_with("autoversion")), "{envelope}");
    assert!(envelope["error_code"].is_string() && envelope["remedy"].is_string() && envelope["context"]["cause"].is_string(), "{envelope}");
    (sentence.to_owned(), envelope)
}

/// The rule's cause the envelope names when `decide` refuses `current` with
/// the two surfaces.
fn rule_cause(current: &str, published: &str, candidate: &str) -> String {
    let envelope = refusal(&autoversion(&["decide", "--current", current, "--published-surface", published, "--candidate-surface", candidate]).stderr).1;
    assert_eq!(envelope["failure_point"], "autoversion.rule", "{envelope}");
    envelope["context"]["cause"].as_str().unwrap().to_owned()
}

fn usage(reason: &str) -> (Option<i32>, String) {
    (Some(USAGE_STATUS), format!("autoversion: error: {reason}\n{USAGE}"))
}

#[test]
fn a_slot_at_the_largest_number_is_refused_rather_than_wrapped() {
    let case = Case::new("exhausted");
    let ab = case.surface("ab.json", &["a", "b"]);
    let abc = case.surface("abc.json", &["a", "b", "c"]);
    let a = case.surface("a.json", &["a"]);
    let largest = u128::MAX;
    let exhausted = |version: String, slot: &str| {
        (Some(REFUSED), format!("refused: {version} cannot advance its {slot} slot: {largest} is the largest number a slot holds; name the next version explicitly"))
    };
    let decide = |current: &str, candidate: &str| refused(&["decide", "--current", current, "--published-surface", &ab, "--candidate-surface", candidate]);

    let major = format!("{largest}.0.0");
    assert_eq!(decide(&major, &a), exhausted(major.clone(), "major"), "a breaking change advances the major slot");
    let patch = format!("1.2.{largest}");
    assert_eq!(decide(&patch, &ab), exhausted(patch.clone(), "patch"), "an internal change advances the patch slot");
    let unstable_minor = format!("0.{largest}.1");
    assert_eq!(decide(&unstable_minor, &a), exhausted(format!("0.{largest}.1"), "minor"), "a breaking change under major 0 advances the minor slot");
    let unstable_patch = format!("0.4.{largest}");
    assert_eq!(decide(&unstable_patch, &abc), exhausted(unstable_patch.clone(), "patch"), "an additive change under major 0 advances the patch slot");
    assert_eq!(rule_cause(&major, &ab, &a), "slot-exhausted");
}

#[test]
fn a_version_it_cannot_read_as_numbers_is_refused_naming_the_slot() {
    let case = Case::new("numeric");
    let ab = case.surface("ab.json", &["a", "b"]);
    let decide = |current: &str| refused(&["decide", "--current", current, "--published-surface", &ab, "--candidate-surface", &ab]);
    let refusal = |current: &str, cause: String| {
        (Some(REFUSED), format!("refused: {current:?} is not numeric in every slot ({cause}), so advancing it would invent an ordering; name the next version explicitly"))
    };

    assert_eq!(decide("1..3"), refusal("1..3", "slot \"\" is not all digits".to_owned()));
    let too_large = "340282366920938463463374607431768211456";
    let error = too_large.parse::<u128>().unwrap_err();
    let current = format!("1.{too_large}.0");
    assert_eq!(decide(&current), refusal(&current, format!("slot {too_large:?} holds no unsigned 128-bit number: {error}")));
    assert_eq!(
        decide("1.2.3.4"),
        (Some(REFUSED), "refused: \"1.2.3.4\" is not a major.minor.patch triple, so there is no slot to advance; name the next version explicitly".to_owned())
    );
    assert_eq!(
        decide("1.2.3+build"),
        (Some(REFUSED), "refused: \"1.2.3+build\" is not a canonical coordinate: expected a non-empty segment of alphanumerics, '.', '_' and '-', with no surrounding whitespace".to_owned())
    );
    for (current, cause) in [("1..3", "not-numeric"), ("1.2.3.4", "not-a-triple"), ("1.2.3+build", "not-canonical")] {
        assert_eq!(rule_cause(current, &ab, &ab), cause, "{current}");
    }
}

#[test]
fn an_empty_candidate_surface_is_refused_and_json_may_follow_the_command() {
    let case = Case::new("candidate");
    let ab = case.surface("ab.json", &["a", "b"]);
    let empty = case.surface("empty.json", &[]);
    assert_eq!(
        refused(&["decide", "--current", "1.2.3", "--published-surface", &ab, "--candidate-surface", &empty]),
        (Some(REFUSED), "refused: the candidate surface is empty, which is far more likely to be a broken extractor than a product that promises nothing".to_owned())
    );
    assert_eq!(rule_cause("1.2.3", &ab, &empty), "empty-surface");

    let duplicated = case.surface("duplicated.json", &["b", "a", "b"]);
    let output = autoversion(&["decide", "--current", "1.2.3", "--published-surface", &ab, "--candidate-surface", &duplicated, "--json"]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        json!({"current": "1.2.3", "change": "internal", "next": "1.2.4", "removed": [], "added": []}),
        "a surface is a set of names: order and repetition change nothing"
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.starts_with("{\n  \"added\": [],\n  \"change\": \"internal\","), "keys sorted and indented: {text}");
}

#[test]
fn every_required_flag_is_named_when_it_is_missing() {
    let case = Case::new("required");
    let ab = case.surface("ab.json", &["a", "b"]);
    assert_eq!(refused(&["decide", "--current", "1.2.3", "--candidate-surface", &ab]), usage("the following argument is required: --published-surface"));
    assert_eq!(refused(&["decide", "--current", "1.2.3", "--published-surface", &ab]), usage("the following argument is required: --candidate-surface"));
    assert_eq!(refused(&["decide", "--published-surface", &ab, "--candidate-surface", &ab]), usage("the following argument is required: --current"));
    assert_eq!(refused(&["decide", "--current"]), usage("--current needs a value"));
    assert_eq!(refused(&["decide", "--module-bindings"]), usage("unrecognized argument: --module-bindings"));

    let init = case.root.join("package.py");
    std::fs::write(&init, "__all__ = [\"a\"]\n").unwrap();
    let init = init.display().to_string();
    assert_eq!(refused(&["check", "--current", "1.2.3", "--candidate-init", &init]), usage("the following argument is required: --published-init"));
    assert_eq!(refused(&["check", "--current", "1.2.3", "--published-init", &init]), usage("the following argument is required: --candidate-init"));
    assert_eq!(refused(&["check", "--published-init", &init, "--candidate-init", &init]), usage("the following argument is required: --current"));
    assert_eq!(refused(&["check", "--expect"]), usage("--expect needs a value"));
    assert_eq!(refused(&["check", "--published-surface", &ab]), usage("unrecognized argument: --published-surface"));

    assert_eq!(refused(&["surface"]), usage("the following argument is required: --python-init"));
    assert_eq!(refused(&["surface", "--python-init"]), usage("--python-init needs a value"));
    assert_eq!(refused(&["surface", "--python-init", &init, "--breaking"]), usage("unrecognized argument: --breaking"));

    assert_eq!(refused(&["order", "--newer", "1.2.3"]), usage("the following argument is required: --older"));
    assert_eq!(refused(&["order", "--older", "1.2.3", "--newer", "1.2.4", "--breaking"]), usage("unrecognized argument: --breaking"));
    assert_eq!(refused(&["--json"]), usage("the following argument is required: command"));
    assert_eq!(refused(&["--json", "--json", "order"]), usage("invalid choice: --json"));
}
