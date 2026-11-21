//! docs/FIXTURES.md, the contract between ports, held against the built
//! `autoversion` as a person runs it: the cases are read the way the fixtures
//! document says a port reads them (the first fenced block, parsed as JSON),
//! and every one is asked of the binary. A classification must name the same
//! change, next version and names; a refusal must refuse (status 1, nothing
//! printed) with the sentence of its kind; an ordering must answer newer one
//! way and not the other. Every disagreement is listed before the case fails.

use std::path::PathBuf;
use std::process::Output;

use serde_json::{json, Value};

/// The words that tell each refusal kind of the fixtures apart in the
/// sentence the binary refuses with.
fn refusal_words(kind: &str) -> &'static str {
    match kind {
        "not-canonical" => "is not a canonical coordinate",
        "not-a-triple" => "is not a major.minor.patch triple",
        "not-numeric" => "is not numeric in every slot",
        "empty-surface" => "surface is empty",
        "slot-exhausted" => "is the largest number a slot holds",
        other => panic!("docs/FIXTURES.md names a refusal kind {other:?} this runner does not know"),
    }
}

fn fixtures() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/FIXTURES.md");
    // failure: a fixtures file that cannot be read fails the case, naming the path and the system's reason.
    let text = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{} cannot be read: {error}", path.display()));
    let block: Vec<&str> = text.split("```").collect();
    // The first fenced block opens with its info string (`json`) on its own line.
    // failure: a document with no fenced block fails the case, naming the path.
    let fenced = block.get(1).unwrap_or_else(|| panic!("{} holds no fenced block", path.display()));
    // fallback: a block with no info-string line is all body, so the whole block is read as JSON.
    let body = fenced.split_once('\n').map_or(*fenced, |(_, rest)| rest);
    // failure: a first block that is not JSON fails the case, naming the path and the parser's reason.
    serde_json::from_str(body).unwrap_or_else(|error| panic!("the first fenced block of {} is not JSON: {error}", path.display()))
}

/// A folder of `area`'s own, emptied: the cases run in parallel.
fn folder(area: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("conformance").join(area);
    match std::fs::remove_dir_all(&root) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => panic!("{} could not be cleared: {error}", root.display()),
    }
    std::fs::create_dir_all(&root).expect("the case's folder is made");
    root
}

fn autoversion(args: &[&str]) -> Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_autoversion")).args(args).output().expect("the binary runs")
}

/// The surface `names` as a surface document under `root`, named after `case` and `side`.
fn surface(root: &PathBuf, case: usize, side: &str, names: &Value) -> String {
    let path = root.join(format!("{case}-{side}.json"));
    std::fs::write(&path, json!({ "surface": names }).to_string()).expect("the surface is written");
    path.display().to_string()
}

fn sorted(value: &Value) -> Vec<String> {
    let mut names: Vec<String> = value.as_array().into_iter().flatten().filter_map(Value::as_str).map(str::to_string).collect();
    names.sort();
    names
}

/// The binary's decision for one case: the JSON it printed, or its status and stderr.
fn decide(root: &PathBuf, index: usize, case: &Value) -> Result<Value, (Option<i32>, String, String)> {
    let published = surface(root, index, "published", &case["published"]);
    let candidate = surface(root, index, "candidate", &case["candidate"]);
    let current = case["current"].as_str().expect("a current version");
    let mut args = vec!["--json", "decide", "--current", current, "--published-surface", published.as_str(), "--candidate-surface", candidate.as_str()];
    if case["declared_breaking"].as_bool() == Some(true) {
        args.push("--breaking");
    }
    let output = autoversion(&args);
    if output.status.success() {
        return Ok(serde_json::from_slice(&output.stdout).expect("the decision is JSON"));
    }
    Err((output.status.code(), String::from_utf8_lossy(&output.stdout).into_owned(), String::from_utf8_lossy(&output.stderr).into_owned()))
}

#[test]
fn every_classification_case_names_the_change_the_next_version_and_the_names() {
    let cases = fixtures();
    let root = folder("classify");
    let mut disagreements = Vec::new();
    for (index, case) in cases["classify"].as_array().expect("classify cases").iter().enumerate() {
        // fallback: a case without a name is listed under an empty name; its disagreement still says what differed.
        let name = case["name"].as_str().unwrap_or_default();
        let expect = &case["expect"];
        match decide(&root, index, case) {
            Ok(decided) => {
                let got = (decided["change"].clone(), decided["next"].clone(), sorted(&decided["removed"]), sorted(&decided["added"]));
                let wanted = (expect["class"].clone(), expect["next"].clone(), sorted(&expect["removed"]), sorted(&expect["added"]));
                if got != wanted {
                    disagreements.push(format!("{name}: decided {got:?}, the fixture says {wanted:?}"));
                }
            }
            Err(refused) => disagreements.push(format!("{name}: refused {refused:?}, the fixture says {expect}")),
        }
    }
    assert!(disagreements.is_empty(), "the binary disagrees with docs/FIXTURES.md:\n{}", disagreements.join("\n"));
}

#[test]
fn every_refusal_case_is_refused_with_the_sentence_of_its_kind() {
    let cases = fixtures();
    let root = folder("refuse");
    let mut disagreements = Vec::new();
    for (index, case) in cases["refuse"].as_array().expect("refusal cases").iter().enumerate() {
        // fallback: a case without a name is listed under an empty name; its disagreement still says what differed.
        let name = case["name"].as_str().unwrap_or_default();
        let kind = case["expect"]["refusal"].as_str().expect("a refusal kind");
        let words = refusal_words(kind);
        match decide(&root, 100 + index, case) {
            Ok(decided) => disagreements.push(format!("{name}: decided {decided}, the fixture says it is refused as {kind}")),
            Err((status, stdout, stderr)) => {
                if status != Some(1) || !stdout.is_empty() || !stderr.starts_with("refused: ") || !stderr.contains(words) {
                    disagreements.push(format!("{name}: status {status:?}, stdout {stdout:?}, stderr {stderr:?}; the fixture says {kind} ({words:?})"));
                }
            }
        }
    }
    assert!(disagreements.is_empty(), "the binary disagrees with docs/FIXTURES.md:\n{}", disagreements.join("\n"));
}

fn is_newer(older: &str, newer: &str) -> String {
    let output = autoversion(&["--json", "order", "--older", older, "--newer", newer]);
    assert!(output.status.success(), "order {older} {newer}: {}", String::from_utf8_lossy(&output.stderr));
    let answered: Value = serde_json::from_slice(&output.stdout).expect("the order is JSON");
    // fallback: an answer without a textual is_newer reads as "", which is neither True nor False, so the case disagrees by name.
    answered["is_newer"].as_str().unwrap_or_default().to_string()
}

#[test]
fn every_ordering_case_answers_newer_one_way_only() {
    let cases = fixtures();
    let mut disagreements = Vec::new();
    for case in cases["order"].as_array().expect("order cases") {
        // fallback: a case without a name is listed under an empty name; its disagreement still says what differed.
        let name = case["name"].as_str().unwrap_or_default();
        let (older, newer) = (case["older"].as_str().expect("older"), case["newer"].as_str().expect("newer"));
        let (forward, backward) = (is_newer(older, newer), is_newer(newer, older));
        if (forward.as_str(), backward.as_str()) != ("True", "False") {
            disagreements.push(format!("{name}: {newer} after {older} is {forward}, the reverse is {backward}"));
        }
    }
    for case in cases["order_equal"].as_array().expect("order_equal cases") {
        // fallback: a case without a name is listed under an empty name; its disagreement still says what differed.
        let name = case["name"].as_str().unwrap_or_default();
        let (left, right) = (case["left"].as_str().expect("left"), case["right"].as_str().expect("right"));
        let (forward, backward) = (is_newer(left, right), is_newer(right, left));
        if (forward.as_str(), backward.as_str()) != ("False", "False") {
            disagreements.push(format!("{name}: {left} and {right} answered {forward} and {backward}"));
        }
    }
    assert!(disagreements.is_empty(), "the binary disagrees with docs/FIXTURES.md:\n{}", disagreements.join("\n"));
}
