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

From Rust the same answer is `autoversion::decide("0.1.0", &published, &candidate, false)`.

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
cargo test --test conformance
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

`autoversion::decide`, `classify` and `Version` are then the rule itself, and
`RuleError::refusal` the names the fixtures use.

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
installed with `cargo install --locked --git https://github.com/lbartoszcze/AutoVersion
--tag <tag> autoversion` once a tag carries it. The commands, flags, answers and exit
codes are the same.

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
