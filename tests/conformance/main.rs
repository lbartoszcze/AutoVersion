//! Check this port against docs/FIXTURES.md.
//!
//! The fixtures are the contract between ports; the compiler keeps nothing
//! honest across languages, so this runner does. It reads the cases the way the
//! fixtures document describes — the first fenced block — so the file stays
//! readable prose with one machine-readable contract inside it.
//!
//! Exits non-zero listing every disagreement it found.

use std::process::ExitCode;

use serde_json::Value;

fn names(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn text<'a>(case: &'a Value, key: &str) -> &'a str {
    case.get(key).and_then(Value::as_str).unwrap_or_default()
}

fn section<'a>(cases: &'a Value, name: &str) -> &'a [Value] {
    cases
        .get(name)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

fn check(cases: &Value) -> Vec<String> {
    let mut failures = Vec::new();
    for case in section(cases, "classify") {
        let name = text(case, "name");
        let declared = case
            .get("declared_breaking")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let got = match autoversion::decide(
            text(case, "current"),
            &names(&case["published"]),
            &names(&case["candidate"]),
            declared,
        ) {
            Ok(got) => got,
            Err(error) => {
                failures.push(format!(
                    "classify {name:?}: refused with {}",
                    error.refusal()
                ));
                continue;
            }
        };
        let want = &case["expect"];
        for (field, expected) in [
            ("change", "class"),
            ("next", "next"),
            ("removed", "removed"),
            ("added", "added"),
        ] {
            if got[field] != want[expected] {
                failures.push(format!(
                    "classify {name:?}: {field} was {}, expected {}",
                    got[field], want[expected]
                ));
            }
        }
    }
    for case in section(cases, "refuse") {
        let name = text(case, "name");
        let expected = case["expect"]["refusal"].as_str().unwrap_or_default();
        match autoversion::decide(
            text(case, "current"),
            &names(&case["published"]),
            &names(&case["candidate"]),
            false,
        ) {
            Ok(_) => failures.push(format!("refuse {name:?}: was accepted")),
            Err(error) if error.refusal() != expected => failures.push(format!(
                "refuse {name:?}: refused with {:?}, expected {expected:?}",
                error.refusal()
            )),
            Err(_) => {}
        }
    }
    for case in section(cases, "order") {
        let (name, older, newer) = (text(case, "name"), text(case, "older"), text(case, "newer"));
        if !autoversion::version_newer(older, newer) {
            failures.push(format!(
                "order {name:?}: {newer:?} did not sort after {older:?}"
            ));
        }
        if autoversion::version_newer(newer, older) {
            failures.push(format!(
                "order {name:?}: the comparison is not strict in one direction"
            ));
        }
    }
    for case in section(cases, "order_equal") {
        if autoversion::version_newer(text(case, "left"), text(case, "right")) {
            failures.push(format!(
                "order_equal {:?}: a version outranked itself",
                text(case, "name")
            ));
        }
    }
    failures
}

fn main() -> ExitCode {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/FIXTURES.md");
    let body = match std::fs::read_to_string(path) {
        Ok(body) => body,
        Err(error) => {
            eprintln!("{path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let Some(block) = body.split("```").nth(1) else {
        eprintln!("{path}: no fenced block");
        return ExitCode::FAILURE;
    };
    let block = block.strip_prefix("json").unwrap_or(block);
    let cases: Value = match serde_json::from_str(block) {
        Ok(cases) => cases,
        Err(error) => {
            eprintln!("{path}: the fenced block is not JSON: {error}");
            return ExitCode::FAILURE;
        }
    };
    let mut total = 0;
    for (name, cases) in cases.as_object().into_iter().flatten() {
        let count = cases.as_array().map(Vec::len).unwrap_or_default();
        total += count;
        println!("  {name}: {count}");
    }
    let failures = check(&cases);
    if failures.is_empty() {
        println!("\nall {total} cases reproduced");
        return ExitCode::SUCCESS;
    }
    println!(
        "\n{} disagreement(s) out of {total} case(s):",
        failures.len()
    );
    for failure in failures {
        println!("  {failure}");
    }
    ExitCode::FAILURE
}
