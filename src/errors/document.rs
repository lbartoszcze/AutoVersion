//! `autoversion.document`: the surface documents `decide` compares, each a
//! JSON object whose "surface" key lists a product's public names.

use wisent_errors::{Cause, Operation};

use super::{DOCS, PLAIN};

pub static DOCUMENT_UNREADABLE: Cause = Cause {
    id: "document-unreadable",
    failed: "reading a surface document decide names",
    cause: "the system refused the read: the path names no file, a directory, or one this user may not read",
    remedy: "name a surface file that exists and this user can read, by a path from the directory autoversion runs in",
    test: "tests/cli/main.rs::decide_refuses_a_version_or_surface_it_would_have_to_invent",
    ..PLAIN
};

pub static DOCUMENT_NOT_JSON: Cause = Cause {
    id: "document-not-json",
    failed: "reading the surface document as JSON",
    cause: "the file does not parse as JSON, so no surface can be read from it",
    remedy: "point at the file the product's surface extraction wrote, a JSON object whose \"surface\" key lists the public names as strings",
    test: "tests/cli/main.rs::decide_refuses_a_version_or_surface_it_would_have_to_invent",
    ..PLAIN
};

pub static SURFACE_KEY_MISSING: Cause = Cause {
    id: "surface-key-missing",
    failed: "finding the surface in the document",
    cause: "the JSON holds no \"surface\" key, the one place decide reads a surface from",
    remedy: "put the public names under the document's \"surface\" key, as a list of strings",
    test: "tests/cli/main.rs::decide_refuses_a_version_or_surface_it_would_have_to_invent",
    ..PLAIN
};

pub static SURFACE_NOT_NAMES: Cause = Cause {
    id: "surface-not-names",
    failed: "reading the names under \"surface\"",
    cause: "\"surface\" is not a list holding only strings, and a surface is a set of public names",
    remedy: "write \"surface\" as a list of strings, one per public name",
    test: "tests/cli/main.rs::decide_refuses_a_version_or_surface_it_would_have_to_invent",
    ..PLAIN
};

pub static DOCUMENT: Operation = Operation {
    id: "autoversion.document",
    entries: &[],
    uses: &[],
    summary: "read a surface document: a JSON object whose \"surface\" key lists a product's public names",
    docs: DOCS,
    causes: &[&DOCUMENT_UNREADABLE, &DOCUMENT_NOT_JSON, &SURFACE_KEY_MISSING, &SURFACE_NOT_NAMES],
    warnings: &[],
};
