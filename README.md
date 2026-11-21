# AutoVersion

One versioning rule, shared by repositories that have nothing else in common.

```sh
$ autoversion decide --current 0.1.0 \
    --published-surface published.json --candidate-surface candidate.json
current: 0.1.0
change: breaking
next: 0.2.0
removed: steer
```

From Rust the same answer is `autoversion::decide("0.1.0", &published, &candidate, false)`:
an `autoversion::Decision`, which always holds all five fields; `Decision::to_json` is
the object `autoversion --json decide` prints.

## What it answers

Given the version currently published, the public surface that was published, and
the surface being proposed: **what kind of change is this, and what is the next
version?**

- a name disappeared → `breaking`
- names were only added → `additive`
- the surfaces match → `internal`

A caller may declare breakage the surface cannot show — a field dropped from a
payload, a stored format changed. A declaration can only escalate the class, never
lower it. Evidence outranks intent.

Which slot moves depends on the current version, because while the major slot is
zero the minor slot carries the compatibility boundary. The full table is in
[`docs/SPEC.md`](docs/SPEC.md).

## What a surface is

A set of public names. Nothing more.

The rule does not care what a name *is*: an entry in `__all__`, an advertised
command, an HTTP route, an exported symbol. **Extraction belongs to the product**,
because only the product knows what it promises. The rule's one requirement is that
both sides were extracted the same way.

```json
{"surface": ["chat", "steer", "evaluate"]}
```

`autoversion surface --python-init FILE` reads direct module-level literal
`__all__` assignments without importing the package. Function and class-local
declarations do not define the module's surface; a later literal module
assignment replaces an earlier one. Definition-time expressions remain visible:
function and lambda defaults, annotations, decorators and class bases can refer
to or mutate the module declaration even though nested bodies are excluded.
Parameter names themselves are local bindings, not module references.
Conditional and augmented assignments are
refused rather than guessed. References to `__all__` outside the literal
assignment target are also refused: method calls, subscripts, deletion and
aliases can change or expose the declared collection without a new literal.
Supply a product-extracted surface when determining the names requires
execution. `--module-bindings` explicitly selects the
different module-binding convention for both trees; it is never an automatic
substitute for a missing `__all__`.

## Why this is a separate repository

Because the consumers have nothing in common except this question.

A Rust control plane, a public Python package, TypeScript applications. Putting the
rule inside any one of them makes it unusable by the others — and the consumer that
matters most is the public package whose version is a contract with strangers, which
cannot depend on an internal binary to compute it.

The alternative is not hypothetical. Before this repository existed the same rule
had been written three times independently, and the copies disagreed about what a
`minor` bump means and about what a breaking change does to a `0.x` version.

## Ports

Each language keeps a small implementation. The compiler cannot keep them honest
across languages, so [`docs/FIXTURES.md`](docs/FIXTURES.md) does:

```sh
kompilator test --manifest-path Cargo.toml --test conformance
```

An implementation is a port when it reproduces every case. Adding a case is a
change to the rule and belongs in the same commit as the ports that satisfy it.

`docs/FIXTURES.md` also records what the suite has caught and, more usefully, one thing
it currently fails to catch.

A Rust product does not need a port: it depends on this crate for the rule
alone, without the Python surface reader the `autoversion` binary uses, and
compiles no parser:

```toml
autoversion = { git = "https://github.com/lbartoszcze/AutoVersion.git", rev = "<commit>", default-features = false }
```

`autoversion::decide` with its `Decision`, `classify` and `Version` are then the
rule itself, and `RuleError::refusal` the names the fixtures use.

## What this repository must never grow

- knowledge of any release channel, storage backend or credential
- knowledge of any product's layout, build command or artifact name
- network or filesystem access beyond the surface a caller handed over
- a second way to answer the same question

## Adopting it

[`docs/adopting.md`](docs/adopting.md) is the operational half: how to decide whether the rule
applies to a repository at all, how to choose a surface, where a baseline comes from,
and the refusals that are correct outcomes rather than failures.

**Read it from `main`.** It postdates the `v0.1.0` tag, so a checkout of the pinned
install does not contain it — three adopting repositories reported that independently.
`v0.1.0` is the Python implementation, installed with pip; `main` is the Rust one,
built from a checkout of a tag that carries it with Stado's `kompilator build
--manifest-path Cargo.toml --release --bin autoversion --out .build/autoversion`
(no Cargo runs). The commands, flags, answers and exit
codes are the same.

## Refusals

Every refusal of `autoversion` is a cause `src/errors` declares for the
functionality that refused. It prints the sentence it has always printed,
`refused: <reason>` (or `autoversion: error: <reason>` and the usage for an
invocation it refuses), then one line of JSON: the
[wisent-errors](https://github.com/wisent-ai/wisent-errors) envelope, with
`failure_point` (the functionality), `error_code`, `retryable`, `detail`
(what failed, why, and the sentence as observed), `remedy` and, under
`context.cause`, the cause's id. A JSON answer on stdout is unchanged; `check`
still prints its decision before it refuses a declared version.

The exit status is the cause's: **2** for an invocation it refuses, **1** for
every other. No cause is retryable: the same files give the same refusal.

`autoversion.decide`, `autoversion.surface` and `autoversion.check` are made
of the layers below and declare a cause of their own only where the tables
say so; `src/errors/check.rs` holds the catalogue to the standard, and every
cause names the test that provokes it (`tests/untested-causes.json` would
list one no local state can provoke, with why; it lists none).

### `autoversion`: the command line

| Cause | Code | Exit | What failed | Why | Remedy |
|---|---|---|---|---|---|
| `command-refused` | refused | 2 | reading the command autoversion is asked to run | no command was named, or the word in its place is not decide, check, surface or order | run `autoversion --help` and name one of the commands it lists: decide, check, surface or order |
| `flag-unrecognized` | refused | 2 | reading the flags after the command | a flag the command does not take was given: each command reads only the flags its own usage line lists | drop the flag the refusal names, or give only the flags `autoversion --help` lists on the command's own line |
| `flag-incomplete` | refused | 2 | reading the values the command's flags hold | a flag the command requires was left out, or a flag that takes a value was the last word, with no value after it | give the flag the refusal names, with its value, as the command's line in `autoversion --help` shows |

### `autoversion.check`: the version the product declares

| Cause | Code | Exit | What failed | Why | Remedy |
|---|---|---|---|---|---|
| `expected-version-unmade` | refused | 1 | holding the version the product declares with --expect to the decision | the surface change requires another next version than the one the product declares; the decision printed on stdout names the version it requires | declare the version the decision names as next, or change the candidate package's public names (or pass --breaking) so the change makes the declared one |

### `autoversion.document`: the surface documents `decide` compares

| Cause | Code | Exit | What failed | Why | Remedy |
|---|---|---|---|---|---|
| `document-unreadable` | refused | 1 | reading a surface document decide names | the system refused the read: the path names no file, a directory, or one this user may not read | name a surface file that exists and this user can read, by a path from the directory autoversion runs in |
| `document-not-json` | refused | 1 | reading the surface document as JSON | the file does not parse as JSON, so no surface can be read from it | point at the file the product's surface extraction wrote, a JSON object whose "surface" key lists the public names as strings |
| `surface-key-missing` | refused | 1 | finding the surface in the document | the JSON holds no "surface" key, the one place decide reads a surface from | put the public names under the document's "surface" key, as a list of strings |
| `surface-not-names` | refused | 1 | reading the names under "surface" | "surface" is not a list holding only strings, and a surface is a set of public names | write "surface" as a list of strings, one per public name |

### `autoversion.python`: a Python package's public names

| Cause | Code | Exit | What failed | Why | Remedy |
|---|---|---|---|---|---|
| `module-unreadable` | refused | 1 | reading the Python module named | the system refused the read: the path names no file, a directory, or one this user may not read | name the package's `__init__.py`, or the module that holds its public names, by a path that exists and this user can read |
| `module-unparseable` | refused | 1 | parsing the Python module | the module holds a syntax error, so its names cannot be read without guessing | correct the syntax error `python -m py_compile` reports for the module |
| `all-referenced` | refused | 1 | reading `__all__` from its literal assignment | `__all__` is mutated, deleted or aliased outside its literal assignment, so what it holds is known only by running the module | assign `__all__` once as a literal list, tuple or set of strings, or have the product extract its own surface and pass it to `autoversion decide` |
| `all-not-direct` | refused | 1 | reading the module-level assignment of `__all__` | `__all__` is assigned under a condition or augmented with `+=`, so which names it holds depends on running the module | assign `__all__` once at module level as a literal list, tuple or set of strings, or have the product extract its own surface and pass it to `autoversion decide` |
| `all-not-literal` | refused | 1 | reading what `__all__` is assigned | `__all__` is assigned something other than a literal list, tuple or set, such as a call or a name, so its contents are known only by executing the module | write `__all__` as a literal list of strings, or name the module bindings as the surface with `--module-bindings` if that is what the package promises |
| `entry-not-string` | refused | 1 | reading an entry of `__all__` | the entry is not a plain string literal: a name, a call, or a string with a b, f or t prefix, interpolation or escape | write each public name in `__all__` as a plain string such as `"name"` |
| `all-missing` | refused | 1 | finding `__all__` in the module | the module declares no `__all__`, so it has no declared public surface to classify a release from | declare `__all__` in the module, pass `--module-bindings` to read its public module-level bindings instead, or supply a surface the product extracts itself to `autoversion decide` |
| `no-public-names` | refused | 1 | reading the module's public bindings | every module-level definition, assignment and re-export begins with an underscore or is a plain import, so the module binds no public name | declare `__all__`, or point at the module that holds the package's public names |

### `autoversion.rule`: the rule

Each cause is one refusal kind of [`docs/FIXTURES.md`](docs/FIXTURES.md), under
the name `RuleError::refusal` gives it.

| Cause | Code | Exit | What failed | Why | Remedy |
|---|---|---|---|---|---|
| `not-canonical` | refused | 1 | reading the current version | the version is not a canonical coordinate: empty, or holding whitespace or a character other than alphanumerics, '.', '_' and '-' | pass `--current` as the version exactly as published, such as 1.2.3, with no surrounding whitespace or build metadata |
| `not-a-triple` | refused | 1 | reading the current version as major.minor.patch | the version does not have exactly three slots, so there is no slot to advance | name the next version explicitly, or pass `--current` as a major.minor.patch version |
| `not-numeric` | refused | 1 | reading each slot of the current version as a number | a slot is not all digits or holds no unsigned 128-bit number, and advancing it would invent an ordering | name the next version explicitly, or pass `--current` as a version whose three slots are numbers |
| `empty-surface` | refused | 1 | reading the published and candidate surfaces | a surface holds no name, which is far more likely a broken extractor than a product that promises nothing | correct the surface extraction so it lists the product's public names, then ask again |
| `slot-exhausted` | refused | 1 | advancing the current version | the slot the change advances already holds the largest number a slot holds, and wrapping it would make a smaller next version | name the next version explicitly |

## A note on its own version

AutoVersion carries no version literal in its manifest; the packaged version comes
from the git tag. That is not a stylistic choice — the workspace this was written in
refuses numeric literals in source files, which is also why the fixtures live inside
a document rather than a `.json` file, and why two cases are still missing.

It is held to its own rule: `released-surface.json` freezes what `v0.1.0` published —
the exported names and the `autoversion` subcommands. The Rust library exports the same
names and the command advertises the same subcommands; what goes is the Python import
path, which is a breaking change for anyone who imported `autoversion` in Python.

## License

MIT.
