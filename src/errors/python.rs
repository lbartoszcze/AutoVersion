//! `autoversion.python`: a Python package's public names, read from its
//! source without importing it: its `__all__`, or with `--module-bindings`
//! its public module-level bindings (`autoversion::surfaces`).

use autoversion::surfaces::SurfaceRefusal;
use wisent_errors::{Cause, Operation};

use super::{DOCS, PLAIN};

pub static MODULE_UNREADABLE: Cause = Cause {
    id: "module-unreadable",
    failed: "reading the Python module named",
    cause: "the system refused the read: the path names no file, a directory, or one this user may not read",
    remedy: "name the package's __init__.py, or the module that holds its public names, by a path that exists and this user can read",
    test: "tests/python/main.rs::surface_refuses_a_package_whose_all_it_cannot_read_without_running_it",
    ..PLAIN
};

pub static MODULE_UNPARSEABLE: Cause = Cause {
    id: "module-unparseable",
    failed: "parsing the Python module",
    cause: "the module holds a syntax error, so its names cannot be read without guessing",
    remedy: "correct the syntax error `python -m py_compile` reports for the module",
    test: "tests/python/main.rs::surface_refuses_a_package_whose_all_it_cannot_read_without_running_it",
    ..PLAIN
};

pub static ALL_REFERENCED: Cause = Cause {
    id: "all-referenced",
    failed: "reading __all__ from its literal assignment",
    cause: "__all__ is mutated, deleted or aliased outside its literal assignment, so what it holds is known only by running the module",
    remedy: "assign __all__ once as a literal list, tuple or set of strings, or have the product extract its own surface and pass it to autoversion decide",
    test: "tests/python/main.rs::surface_refuses_a_package_whose_all_it_cannot_read_without_running_it",
    ..PLAIN
};

pub static ALL_NOT_DIRECT: Cause = Cause {
    id: "all-not-direct",
    failed: "reading the module-level assignment of __all__",
    cause: "__all__ is assigned under a condition or augmented with +=, so which names it holds depends on running the module",
    remedy: "assign __all__ once at module level as a literal list, tuple or set of strings, or have the product extract its own surface and pass it to autoversion decide",
    test: "tests/python/main.rs::surface_refuses_a_package_whose_all_it_cannot_read_without_running_it",
    ..PLAIN
};

pub static ALL_NOT_LITERAL: Cause = Cause {
    id: "all-not-literal",
    failed: "reading what __all__ is assigned",
    cause: "__all__ is assigned something other than a literal list, tuple or set, such as a call or a name, so its contents are known only by executing the module",
    remedy: "write __all__ as a literal list of strings, or name the module bindings as the surface with --module-bindings if that is what the package promises",
    test: "tests/cli/main.rs::check_reads_two_python_packages_and_refuses_a_declared_version_the_change_does_not_make",
    ..PLAIN
};

pub static ENTRY_NOT_STRING: Cause = Cause {
    id: "entry-not-string",
    failed: "reading an entry of __all__",
    cause: "the entry is not a plain string literal: a name, a call, or a string with a b, f or t prefix, interpolation or escape",
    remedy: "write each public name in __all__ as a plain string such as \"name\"",
    test: "tests/python/main.rs::surface_refuses_a_package_whose_all_it_cannot_read_without_running_it",
    ..PLAIN
};

pub static ALL_MISSING: Cause = Cause {
    id: "all-missing",
    failed: "finding __all__ in the module",
    cause: "the module declares no __all__, so it has no declared public surface to classify a release from",
    remedy: "declare __all__ in the module, pass --module-bindings to read its public module-level bindings instead, or supply a surface the product extracts itself to autoversion decide",
    test: "tests/cli/main.rs::check_reads_two_python_packages_and_refuses_a_declared_version_the_change_does_not_make",
    ..PLAIN
};

pub static NO_PUBLIC_NAMES: Cause = Cause {
    id: "no-public-names",
    failed: "reading the module's public bindings",
    cause: "every module-level definition, assignment and re-export begins with an underscore or is a plain import, so the module binds no public name",
    remedy: "declare __all__, or point at the module that holds the package's public names",
    test: "tests/python/main.rs::module_bindings_are_the_public_names_a_module_binds",
    ..PLAIN
};

pub static PYTHON: Operation = Operation {
    id: "autoversion.python",
    entries: &[],
    uses: &[],
    summary: "read a Python package's public names from its source without importing it: its __all__, or its public module-level bindings",
    docs: DOCS,
    causes: &[
        &MODULE_UNREADABLE,
        &MODULE_UNPARSEABLE,
        &ALL_REFERENCED,
        &ALL_NOT_DIRECT,
        &ALL_NOT_LITERAL,
        &ENTRY_NOT_STRING,
        &ALL_MISSING,
        &NO_PUBLIC_NAMES,
    ],
    warnings: &[],
};

/// The cause the reader's refusal of `refusal` is.
pub fn cause(refusal: SurfaceRefusal) -> &'static Cause {
    match refusal {
        SurfaceRefusal::Unreadable => &MODULE_UNREADABLE,
        SurfaceRefusal::Unparseable => &MODULE_UNPARSEABLE,
        SurfaceRefusal::AllReferenced => &ALL_REFERENCED,
        SurfaceRefusal::AllNotDirect => &ALL_NOT_DIRECT,
        SurfaceRefusal::AllNotLiteral => &ALL_NOT_LITERAL,
        SurfaceRefusal::EntryNotString => &ENTRY_NOT_STRING,
        SurfaceRefusal::AllMissing => &ALL_MISSING,
        SurfaceRefusal::NoPublicNames => &NO_PUBLIC_NAMES,
    }
}
