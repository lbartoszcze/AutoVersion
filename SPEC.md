# The versioning rule

One rule, several implementations, kept identical by shared fixtures.

This repository holds no product logic, no release channel, no git guards and no
network access. It answers exactly one question:

> Given the version currently published, the public surface that was published,
> and the public surface being proposed — what kind of change is this, and what
> is the next version?

Everything else belongs to the product: how a surface is extracted, where an
artifact is published, who is allowed to publish it.

## Why this is not a library in one language

The consumers are not in one language. A Rust control plane, a public Python
package, TypeScript applications. A single package would force its runtime onto
every consumer, and the consumer that matters most — the public `pip` package
whose version is a contract with strangers — cannot depend on an internal
binary.

So the rule is specified here, and each language keeps a small implementation
that must pass the fixtures in `fixtures.json`. The compiler does not keep the
ports honest. The fixtures do.

That cost is deliberate and it is the smaller cost. The alternative already
happened: the same rule was written three times independently, and the copies
disagree.

## The surface

A surface is a set of public names. Nothing more.

The rule does not care what a name *is*. For a Python package it is an entry in
`__all__`. For a command-line tool it is an advertised command. For a service it
is a route. For a library with a C ABI it is an exported symbol.

Extraction is the product's job, because only the product knows what it promises
to callers. The rule's only requirement is that both sides of a comparison are
extracted **the same way** — comparing exported symbols against advertised
commands answers nothing.

Surfaces are exchanged as JSON:

```json
{"surface": ["chat", "steer", "evaluate"]}
```

Order is irrelevant; duplicates are collapsed. An empty surface is refused
rather than treated as "everything was removed", because an extractor that
silently produced nothing is the most likely explanation.

## The classes

A change is classified from the difference between the two surfaces:

| Condition | Class |
| --- | --- |
| a name present in the published surface is absent from the candidate | `breaking` |
| no name was removed, and at least one was added | `additive` |
| the surfaces are equal | `internal` |

A caller may additionally *declare* breakage the surface cannot show: a field
dropped from a payload, a stored format changed, an exit code repurposed.

A declaration may only **escalate** the class. There is deliberately no way to
lower it: a removed name cannot be published as a fix by asserting that it is
fine. Evidence outranks intent.

## What moves

The class says what happened. Which slot advances is a separate question,
answered by the version currently published:

| Class | while major is zero | once major is at least one |
| --- | --- | --- |
| `breaking` | minor advances, patch resets | major advances, minor and patch reset |
| `additive` | patch advances | minor advances, patch resets |
| `internal` | patch advances | patch advances |

While the major slot is zero, the minor slot carries the compatibility boundary.
This follows the semantic-versioning text for `0.y.z` releases and matches how
Cargo resolves such versions; PyPI resolvers do not contradict it.

The user's own framing of the question was whether a change takes `0.1.0` to
`0.1.1` or to `0.2.0`. Under this table: `breaking` gives `0.2.0`, and both
`additive` and `internal` give `0.1.1`.

**Under `0.x` an additive change and an internal one land in the same slot.**
That is not an oversight. A `0.x` release has no third slot, and both changes
are compatible for a consumer. The class is still reported separately, so the
reason for a release survives a number that cannot express it.

## What is refused

The rule refuses rather than guesses:

- **A version that is not three numeric slots.** Real coordinates in this fleet
  include upstream repackagings and incident labels that are not versions at
  all. Advancing one would mean inventing which slot is the minor, so the caller
  is told to name the next version explicitly.
- **A version carrying surrounding whitespace, or characters outside
  alphanumerics, `.`, `_` and `-`.** Such a string cannot survive a URL path and
  a filesystem key unchanged.
- **An empty surface on either side.**

Refusals name the reason, never a fault, and never a substitute value.

## Ordering

Comparing two versions is separate from advancing one, and it works on strings
that are not triples, because ordering is needed for coordinates that can never
be bumped.

Split on `.` and `-`. A token that parses as an integer compares as an integer;
any other token compares as a string; a numeric token sorts before a string
token. Compare the resulting sequences element by element, so a proper prefix
sorts before its extension.

This matches the Python tuple comparison the fleet's existing implementations
already used, and is preserved here so no consumer's ordering changes.

## Conformance

An implementation is correct when it produces, for every case in
`fixtures.json`, exactly the recorded class, next version, and refusal.

Fixtures are the contract between ports. A port that cannot pass them is not a
port of this rule. Adding a case is a change to the rule and belongs in the same
commit as the ports that satisfy it.

## What this repository must never grow

- Knowledge of any release channel, storage backend or credential.
- Knowledge of any product's layout, build command or artifact name.
- Network or filesystem access beyond reading a surface a caller handed over.
- A second way to answer the same question.
