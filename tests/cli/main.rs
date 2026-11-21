//! `autoversion decide`, `check`, `surface`, `order` and `help` through the
//! built binary on files of the case's own: each decision the rule makes,
//! as JSON and as lines, and every refusal whole with its status (1 when the
//! rule declined to invent an answer, 2 when the invocation is wrong).

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
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("cli-{name}"));
        // failure: nothing to clear on a first run; a folder an earlier run left that cannot be removed holds only this case's files, each written over.
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn file(&self, name: &str, body: &str) -> String {
        let path = self.root.join(name);
        std::fs::write(&path, body).unwrap();
        path.display().to_string()
    }

    fn surface(&self, name: &str, names: &[&str]) -> String {
        self.file(name, &json!({"surface": names}).to_string())
    }
}

fn autoversion(args: &[&str]) -> Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_autoversion")).args(args).output().expect("the binary runs")
}

fn answered(args: &[&str]) -> Value {
    let output = autoversion(args);
    assert!(output.status.success(), "{args:?}: {}", String::from_utf8_lossy(&output.stderr));
    serde_json::from_slice(&output.stdout).unwrap()
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

fn usage(reason: &str) -> (Option<i32>, String) {
    (Some(USAGE_STATUS), format!("autoversion: error: {reason}\n{USAGE}"))
}

#[test]
fn decide_classifies_the_change_and_names_the_next_version() {
    let case = Case::new("decide");
    let ab = case.surface("ab.json", &["a", "b"]);
    let abc = case.surface("abc.json", &["a", "b", "c"]);
    let a = case.surface("a.json", &["a"]);
    let decide = |current: &str, published: &str, candidate: &str, more: &[&str]| {
        let mut args = vec!["--json", "decide", "--current", current, "--published-surface", published, "--candidate-surface", candidate];
        args.extend(more);
        answered(&args)
    };
    assert_eq!(decide("1.2.3", &ab, &abc, &[]), json!({"current": "1.2.3", "change": "additive", "next": "1.3.0", "removed": [], "added": ["c"]}));
    assert_eq!(decide("1.2.3", &ab, &a, &[]), json!({"current": "1.2.3", "change": "breaking", "next": "2.0.0", "removed": ["b"], "added": []}));
    assert_eq!(decide("1.2.3", &ab, &ab, &[])["next"], "1.2.4", "the same surface is an internal change");
    assert_eq!(decide("1.2.3", &ab, &ab, &["--breaking"])["next"], "2.0.0", "a declaration only escalates");
    assert_eq!(decide("0.4.1", &ab, &a, &[])["next"], "0.5.0", "a breaking change under major 0 advances the minor slot");
    assert_eq!(decide("0.4.1", &ab, &abc, &[])["next"], "0.4.2", "an additive change under major 0 advances the patch slot");

    let lines = autoversion(&["decide", "--current", "1.2.3", "--published-surface", &ab, "--candidate-surface", &abc]);
    assert_eq!(String::from_utf8(lines.stdout).unwrap(), "current: 1.2.3\nchange: additive\nnext: 1.3.0\nadded: c\n", "a field that holds nothing is not printed");
}

#[test]
fn decide_refuses_a_version_or_surface_it_would_have_to_invent() {
    let case = Case::new("decide-refusals");
    let ab = case.surface("ab.json", &["a", "b"]);
    let abc = case.surface("abc.json", &["a", "b", "c"]);
    let decide = |current: &str, published: &str| refused(&["decide", "--current", current, "--published-surface", published, "--candidate-surface", &abc]);
    assert_eq!(
        decide(" 1.2.3", &ab),
        (Some(REFUSED), "refused: \" 1.2.3\" is not a canonical coordinate: expected a non-empty segment of alphanumerics, '.', '_' and '-', with no surrounding whitespace".to_owned())
    );
    assert_eq!(decide("v1", &ab), (Some(REFUSED), "refused: \"v1\" is not a major.minor.patch triple, so there is no slot to advance; name the next version explicitly".to_owned()));
    let triple = refusal(&autoversion(&["decide", "--current", "v1", "--published-surface", &ab, "--candidate-surface", &abc]).stderr).1;
    assert_eq!(
        (triple["failure_point"].as_str(), triple["error_code"].as_str(), triple["context"]["cause"].as_str()),
        (Some("autoversion.rule"), Some("refused"), Some("not-a-triple")),
        "{triple}"
    );
    assert!(
        triple["detail"].as_str().is_some_and(|detail| detail.ends_with("; observed: \"v1\" is not a major.minor.patch triple, so there is no slot to advance; name the next version explicitly")),
        "the envelope's observed state is the refusal as the caller read it: {triple}"
    );
    assert_eq!(
        decide("1.x.3", &ab),
        (Some(REFUSED), "refused: \"1.x.3\" is not numeric in every slot (slot \"x\" is not all digits), so advancing it would invent an ordering; name the next version explicitly".to_owned())
    );
    let largest = u128::MAX;
    assert_eq!(
        decide(&format!("1.{largest}.0"), &ab),
        (Some(REFUSED), format!("refused: 1.{largest}.0 cannot advance its minor slot: {largest} is the largest number a slot holds; name the next version explicitly"))
    );
    let empty = case.surface("empty.json", &[]);
    assert_eq!(
        decide("1.2.3", &empty),
        (Some(REFUSED), "refused: the published surface is empty, which is far more likely to be a broken extractor than a product that promises nothing".to_owned())
    );

    let missing = case.root.join("absent.json").display().to_string();
    let cause = std::fs::read_to_string(&missing).unwrap_err();
    assert_eq!(decide("1.2.3", &missing), (Some(REFUSED), format!("refused: {missing}: {cause}")));
    let garbage = case.file("garbage.json", "not json");
    let error = serde_json::from_str::<Value>("not json").unwrap_err();
    assert_eq!(decide("1.2.3", &garbage), (Some(REFUSED), format!("refused: {garbage}: not JSON: {error}")));
    let keyless = case.file("keyless.json", "{}");
    assert_eq!(
        decide("1.2.3", &keyless),
        (Some(REFUSED), format!("refused: {keyless}: no \"surface\" key. A surface document is {{\"surface\": [\"name\", ...]}}"))
    );
    let numbers = case.file("numbers.json", "{\"surface\": [1, 2]}");
    assert_eq!(decide("1.2.3", &numbers), (Some(REFUSED), format!("refused: {numbers}: \"surface\" is not a list of names")));
    let unkeyed = refusal(&autoversion(&["decide", "--current", "1.2.3", "--published-surface", &keyless, "--candidate-surface", &abc]).stderr).1;
    assert_eq!((unkeyed["failure_point"].as_str(), unkeyed["context"]["cause"].as_str()), (Some("autoversion.document"), Some("surface-key-missing")), "{unkeyed}");
}

#[test]
fn check_reads_two_python_packages_and_refuses_a_declared_version_the_change_does_not_make() {
    let case = Case::new("check");
    let published = case.file("published.py", "__all__ = [\"a\", \"b\"]\n");
    let candidate = case.file("candidate.py", "__all__ = (\"a\", \"b\", \"c\")\n");
    let check = |more: &[&str]| {
        let mut args = vec!["--json", "check", "--current", "1.2.3", "--published-init", published.as_str(), "--candidate-init", candidate.as_str()];
        args.extend(more);
        autoversion(&args)
    };
    let output = check(&["--expect", "1.3.0"]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        json!({"current": "1.2.3", "change": "additive", "next": "1.3.0", "removed": [], "added": ["c"]})
    );
    let wrong = check(&["--expect", "1.2.4"]);
    assert_eq!(wrong.status.code(), Some(REFUSED), "the decision is printed, then the declared version refused");
    assert_eq!(serde_json::from_slice::<Value>(&wrong.stdout).unwrap()["next"], "1.3.0");
    let (sentence, envelope) = refusal(&wrong.stderr);
    assert_eq!(sentence, "refused: the surface change requires 1.3.0, but the product declares 1.2.4");
    assert_eq!((envelope["failure_point"].as_str(), envelope["context"]["cause"].as_str()), (Some("autoversion.check"), Some("expected-version-unmade")), "{envelope}");

    let undeclared = case.file("undeclared.py", "def a():\n    pass\n");
    assert_eq!(
        refused(&["check", "--current", "1.2.3", "--published-init", &published, "--candidate-init", &undeclared]),
        (
            Some(REFUSED),
            format!("refused: {undeclared}: no __all__ found. A package without one has no declared public surface, so a release cannot be classified from it — declare __all__, or supply a surface the product extracts itself; name the module bindings as the surface with --module-bindings if that is what the package promises")
        )
    );
    let computed = case.file("computed.py", "__all__ = names()\n");
    assert_eq!(
        refused(&["check", "--current", "1.2.3", "--published-init", &published, "--candidate-init", &computed]),
        (
            Some(REFUSED),
            format!("refused: {computed}: __all__ is not a literal list, tuple or set, so its contents cannot be read without executing the module; name the module bindings as the surface with --module-bindings if that is what the package promises")
        )
    );
}

#[test]
fn surface_prints_a_package_s_declared_names_or_its_module_bindings() {
    let case = Case::new("surface");
    let declared = case.file("declared.py", "__all__ = [\"b\", \"a\"]\n");
    assert_eq!(answered(&["surface", "--python-init", &declared]), json!({"surface": ["b", "a"]}));
    let bound = case.file("bound.py", "import os\nfrom pkg import helper\n\ndef run():\n    pass\n\n_hidden = 1\nLIMIT = 3\n");
    let surface = answered(&["surface", "--python-init", &bound, "--module-bindings"]);
    assert_eq!(surface["source"], "module bindings");
    let mut names: Vec<&str> = surface["surface"].as_array().unwrap().iter().map(|name| name.as_str().unwrap()).collect();
    names.sort();
    assert_eq!(names, ["LIMIT", "helper", "run"], "public definitions, assignments and re-exports, not a plain import or a private name");
}

#[test]
fn order_says_whether_one_version_is_newer() {
    assert_eq!(
        answered(&["--json", "order", "--older", "1.2.3", "--newer", "1.10.0"]),
        json!({"older": "1.2.3", "newer": "1.10.0", "is_newer": "True"})
    );
    assert_eq!(answered(&["--json", "order", "--older", "1.10.0", "--newer", "1.2.3"])["is_newer"], "False");
    assert_eq!(String::from_utf8(autoversion(&["order", "--older", "1.2.3", "--newer", "1.2.4"]).stdout).unwrap(), "older: 1.2.3\nnewer: 1.2.4\nis_newer: True\n");
}

#[test]
fn a_wrong_invocation_is_refused_with_the_usage() {
    for args in [&["--help"][..], &["-h"], &["help"], &["--json", "--help"], &["decide", "--help"], &["check", "-h"], &["surface", "--help"], &["order", "--older", "1.2.3", "--help"]] {
        let help = autoversion(args);
        assert_eq!((help.status.code(), String::from_utf8(help.stdout).unwrap()), (Some(0), format!("{USAGE}\n")), "{args:?}: --help is answered at every level");
    }
    assert_eq!(refused(&[]), usage("the following argument is required: command"));
    assert_eq!(refused(&["frob"]), usage("invalid choice: frob"));
    assert_eq!(refused(&["order", "--older", "1.2.3"]), usage("the following argument is required: --newer"));
    assert_eq!(refused(&["order", "--older"]), usage("--older needs a value"));
    assert_eq!(refused(&["order", "--sideways", "1"]), usage("unrecognized argument: --sideways"));
    for (args, cause) in [(&["frob"][..], "command-refused"), (&["order", "--sideways", "1"][..], "flag-unrecognized"), (&["order", "--older"][..], "flag-incomplete")] {
        let envelope = refusal(&autoversion(args).stderr).1;
        assert_eq!((envelope["failure_point"].as_str(), envelope["context"]["cause"].as_str()), (Some("autoversion"), Some(cause)), "{args:?}: {envelope}");
    }
}
