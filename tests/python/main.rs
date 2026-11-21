//! `autoversion surface` and `autoversion check` reading Python packages of
//! the case's own through the built binary: every name `__all__` and the
//! module bindings yield, and every way a package's surface is refused, whole,
//! with status 1 and nothing printed. `check` reports the module bindings it
//! compared, takes a declared breaking change, and answers as lines.

use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::Output;

const REFUSED: i32 = 1;
const BINDINGS_HINT: &str = "; name the module bindings as the surface with --module-bindings if that is what the package promises";

struct Case {
    root: PathBuf,
}

impl Case {
    fn new(name: &str) -> Self {
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("python-{name}"));
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

/// The declared cause `surface` names when it refuses `path`.
fn surface_cause(path: &str, bindings: bool) -> String {
    let mut args = vec!["surface", "--python-init", path];
    if bindings {
        args.push("--module-bindings");
    }
    let envelope = refusal(&autoversion(&args).stderr).1;
    assert_eq!(envelope["failure_point"], "autoversion.python", "{envelope}");
    envelope["context"]["cause"].as_str().unwrap().to_owned()
}

/// `surface` refusing `path`, read by `__all__` (the hint to read the module
/// bindings instead is part of the sentence) or by its bindings.
fn surface_refused(path: &str, bindings: bool) -> (Option<i32>, String) {
    let mut args = vec!["surface", "--python-init", path];
    if bindings {
        args.push("--module-bindings");
    }
    refused(&args)
}

#[test]
fn surface_refuses_a_package_whose_all_it_cannot_read_without_running_it() {
    let case = Case::new("all-refusals");
    let refusal = |reason: String| (Some(REFUSED), format!("refused: {reason}{BINDINGS_HINT}"));

    let missing = case.root.join("absent.py").display().to_string();
    let cause = std::fs::read_to_string(&missing).unwrap_err();
    assert_eq!(surface_refused(&missing, false), refusal(format!("{missing}: {cause}")));
    assert_eq!(surface_cause(&missing, false), "module-unreadable");

    let broken = case.file("broken.py", "__all__ = [\n");
    assert_eq!(
        surface_refused(&broken, false),
        refusal(format!("{broken}: not parseable Python, so its names cannot be read; correct the syntax error `python -m py_compile` reports for it"))
    );
    assert_eq!(surface_cause(&broken, false), "module-unparseable");

    for (name, body) in [("appended.py", "__all__ = [\"a\"]\n__all__.append(\"b\")\n"), ("aliased.py", "__all__ = [\"a\"]\nnames = __all__\n"), ("deleted.py", "__all__ = [\"a\"]\ndel __all__\n")] {
        let path = case.file(name, body);
        assert_eq!(
            surface_refused(&path, false),
            refusal(format!("{path}: __all__ is referenced outside a literal assignment; mutation, deletion or aliasing requires a product-extracted surface")),
            "{name}"
        );
        assert_eq!(surface_cause(&path, false), "all-referenced", "{name}");
    }

    for (name, body) in [("conditional.py", "import sys\nif sys.version_info >= (3, 12):\n    __all__ = [\"a\"]\n"), ("augmented.py", "__all__ = [\"a\"]\n__all__ += [\"b\"]\n")] {
        let path = case.file(name, body);
        assert_eq!(
            surface_refused(&path, false),
            refusal(format!("{path}: __all__ must be a direct module-level literal assignment; conditional or augmented assignments require a product-extracted surface")),
            "{name}"
        );
        assert_eq!(surface_cause(&path, false), "all-not-direct", "{name}");
    }

    for (name, body, entry) in [("named.py", "__all__ = [\"a\", helper]\n", "helper"), ("formatted.py", "__all__ = [f\"a\"]\n", "f\"a\""), ("bytes.py", "__all__ = (b\"a\",)\n", "b\"a\"")] {
        let path = case.file(name, body);
        assert_eq!(
            surface_refused(&path, false),
            refusal(format!("{path}: __all__ contains {entry}, which is not a string literal; write each public name as a plain string such as \"name\", with no b, f or t prefix, interpolation or escape")),
            "{name}"
        );
        assert_eq!(surface_cause(&path, false), "entry-not-string", "{name}");
    }
}

#[test]
fn surface_reads_every_form_of_a_literal_all() {
    let case = Case::new("all-forms");
    let set = case.file("set.py", "__all__ = {\"a\"}\n");
    assert_eq!(answered(&["surface", "--python-init", &set]), json!({"surface": ["a"]}));
    let bare = case.file("bare.py", "__all__ = \"a\", \"b\"  # the names\n");
    assert_eq!(answered(&["--json", "surface", "--python-init", &bare]), json!({"surface": ["a", "b"]}));
    let commented = case.file("commented.py", "__all__ = [\n    \"a\",  # first\n    'b',\n]\n\ndef helper():\n    __all__ = [\"inside\"]\n");
    assert_eq!(answered(&["surface", "--python-init", &commented, "--json"]), json!({"surface": ["a", "b"]}), "a comment is not an entry and a function body is not module scope");
    let reassigned = case.file("reassigned.py", "__all__ = [\"old\"]\n__all__ = [\"new\"]\n");
    assert_eq!(answered(&["surface", "--python-init", &reassigned]), json!({"surface": ["new"]}), "the last module-level assignment holds");
}

#[test]
fn module_bindings_are_the_public_names_a_module_binds() {
    let case = Case::new("bindings");
    let module = case.file(
        "module.py",
        "from pkg import helper as public_helper, _private\nimport json\n\n@decorated\nclass Engine:\n    inner = 1\n\nFIRST = SECOND = 2\n\ndef run():\n    nested = 3\n\n_hidden = 4\n",
    );
    assert_eq!(
        answered(&["surface", "--python-init", &module, "--module-bindings"]),
        json!({"surface": ["public_helper", "Engine", "FIRST", "SECOND", "run"], "source": "module bindings"}),
        "in source order, once each"
    );

    let private = case.file("private.py", "import os\n_cache = {}\n\ndef _helper():\n    pass\n");
    assert_eq!(
        surface_refused(&private, true),
        (Some(REFUSED), format!("refused: {private}: no public names found, with or without __all__: every module-level definition, assignment and re-export begins with an underscore or is a plain import; declare __all__, or point at the module that holds the package's public names"))
    );
    assert_eq!(surface_cause(&private, true), "no-public-names");
    let missing = case.root.join("absent.py").display().to_string();
    let cause = std::fs::read_to_string(&missing).unwrap_err();
    assert_eq!(surface_refused(&missing, true), (Some(REFUSED), format!("refused: {missing}: {cause}")), "read by its bindings, the hint to read them is not offered");
    let broken = case.file("broken.py", "def run(:\n");
    assert_eq!(
        surface_refused(&broken, true),
        (Some(REFUSED), format!("refused: {broken}: not parseable Python, so its names cannot be read; correct the syntax error `python -m py_compile` reports for it"))
    );
}

#[test]
fn check_compares_module_bindings_takes_a_declared_break_and_answers_as_lines() {
    let case = Case::new("check");
    let published = case.file("published.py", "def a():\n    pass\n\ndef b():\n    pass\n");
    let candidate = case.file("candidate.py", "def a():\n    pass\n\ndef b():\n    pass\n\ndef c():\n    pass\n");
    let check = |more: &[&str]| {
        let mut args = vec!["check", "--current", "1.2.3", "--published-init", published.as_str(), "--candidate-init", candidate.as_str()];
        args.extend(more);
        autoversion(&args)
    };

    let bindings = check(&["--module-bindings", "--json"]);
    assert!(bindings.status.success(), "{}", String::from_utf8_lossy(&bindings.stderr));
    assert_eq!(
        serde_json::from_slice::<Value>(&bindings.stdout).unwrap(),
        json!({"current": "1.2.3", "change": "additive", "next": "1.3.0", "removed": [], "added": ["c"], "surface": "module bindings"})
    );

    let declared = check(&["--module-bindings", "--breaking", "--expect", "2.0.0"]);
    assert_eq!(declared.status.code(), Some(0), "{}", String::from_utf8_lossy(&declared.stderr));
    assert_eq!(
        String::from_utf8(declared.stdout).unwrap(),
        "current: 1.2.3\nchange: breaking\nnext: 2.0.0\nadded: c\nsurface: module bindings\n",
        "a declaration escalates an additive change; a field that holds nothing is not printed"
    );

    let undeclared = check(&[]);
    assert_eq!(
        (undeclared.status.code(), refusal(&undeclared.stderr).0),
        (
            Some(REFUSED),
            format!("refused: {published}: no __all__ found. A package without one has no declared public surface, so a release cannot be classified from it — declare __all__, or supply a surface the product extracts itself{BINDINGS_HINT}")
        ),
        "a package read by __all__ is never read by its bindings instead"
    );
    assert!(undeclared.stdout.is_empty());

    let wrong = check(&["--module-bindings", "--expect", "1.2.4"]);
    assert_eq!(wrong.status.code(), Some(REFUSED));
    assert_eq!(String::from_utf8(wrong.stdout).unwrap(), "current: 1.2.3\nchange: additive\nnext: 1.3.0\nadded: c\nsurface: module bindings\n", "the decision is printed before the declared version is refused");
    assert_eq!(refusal(&wrong.stderr).0, "refused: the surface change requires 1.3.0, but the product declares 1.2.4");
}
