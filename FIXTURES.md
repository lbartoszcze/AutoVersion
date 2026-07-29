# Conformance fixtures

The contract between ports. An implementation is a port of this rule when it
reproduces every case below exactly: the same class, the same next version, the
same refusal.

The cases live in the fenced block, so a port reads them without a parser of its
own: take the file, keep what lies between the first pair of fences, parse it as
JSON. Prose around the block explains why each case exists; the block is the
contract.

Adding a case is a change to the rule. It belongs in the same commit as the ports
that satisfy it.

## Why the numbers are what they are

The unstable-major half of the slot table is fixed by the question this rule
exists to answer, asked in exactly these terms: *"potrzebujemy jasnej logiki tego
kiedy ktora wersja sie zmienia np. 0.1.0 na 0.1.1. czy 0.2.0"*. So `0.1.0` is the
base, `0.1.1` is where a compatible change lands, and `0.2.0` is where a breaking
one lands.

The stable half is fixed the same way, by the operator naming the pair in exactly
these terms: *"przy 1.2.3 zmiana łamiąca daje 2.0.0, a rozszerzająca 1.3.0"*. So
`1.2.3` is the stable base, `2.0.0` is where a breaking change lands, and `1.3.0`
is where an additive one lands.

Both halves therefore rest on a sentence a human wrote, not on a convention an
author assumed. That is the point of recording them here.

Refusal cases deliberately avoid version-shaped strings where a digit-free one
proves the same thing: `x.y.z` has three slots and no numeric one, `latest` is a
legal path segment that is not a version. That keeps the suite honest about what
it is testing — the shape of the input, not the digits in it.

## What is not covered yet

One case: what an `internal` change produces at a stable major. Both halves of the
slot table are otherwise proven. The reason for that single hole is the same as for
the numbers themselves and is stated in the closing section.

```json
{
  "classify": [
    {
      "name": "a removed name is breaking, and while major is zero it advances minor",
      "current": "0.1.0",
      "published": ["chat", "steer", "evaluate"],
      "candidate": ["chat", "evaluate"],
      "declared_breaking": false,
      "expect": {"class": "breaking", "next": "0.2.0", "removed": ["steer"], "added": []}
    },
    {
      "name": "an added name is additive, and while major is zero it advances patch",
      "current": "0.1.0",
      "published": ["chat", "steer"],
      "candidate": ["chat", "steer", "evaluate"],
      "declared_breaking": false,
      "expect": {"class": "additive", "next": "0.1.1", "removed": [], "added": ["evaluate"]}
    },
    {
      "name": "an identical surface is internal, and order does not matter",
      "current": "0.1.0",
      "published": ["chat", "steer"],
      "candidate": ["steer", "chat"],
      "declared_breaking": false,
      "expect": {"class": "internal", "next": "0.1.1", "removed": [], "added": []}
    },
    {
      "name": "duplicates in a surface collapse",
      "current": "0.1.0",
      "published": ["steer", "chat", "chat"],
      "candidate": ["chat", "steer"],
      "declared_breaking": false,
      "expect": {"class": "internal", "next": "0.1.1", "removed": [], "added": []}
    },
    {
      "name": "a removal is breaking even when something was added alongside it",
      "current": "0.1.0",
      "published": ["chat", "steer"],
      "candidate": ["chat", "evaluate"],
      "declared_breaking": false,
      "expect": {"class": "breaking", "next": "0.2.0", "removed": ["steer"], "added": ["evaluate"]}
    },
    {
      "name": "a declaration escalates an internal change",
      "current": "0.1.0",
      "published": ["chat"],
      "candidate": ["chat"],
      "declared_breaking": true,
      "expect": {"class": "breaking", "next": "0.2.0", "removed": [], "added": []}
    },
    {
      "name": "a declaration escalates an additive change",
      "current": "0.1.0",
      "published": ["chat"],
      "candidate": ["chat", "steer"],
      "declared_breaking": true,
      "expect": {"class": "breaking", "next": "0.2.0", "removed": [], "added": ["steer"]}
    },
    {
      "name": "an unset declaration cannot lower a breaking change",
      "current": "0.1.0",
      "published": ["chat", "steer"],
      "candidate": ["chat"],
      "declared_breaking": false,
      "expect": {"class": "breaking", "next": "0.2.0", "removed": ["steer"], "added": []}
    },
    {
      "name": "a removed name at a stable major advances major, resetting the rest",
      "current": "1.2.3",
      "published": ["chat", "steer"],
      "candidate": ["chat"],
      "declared_breaking": false,
      "expect": {"class": "breaking", "next": "2.0.0", "removed": ["steer"], "added": []}
    },
    {
      "name": "an added name at a stable major advances minor, resetting patch",
      "current": "1.2.3",
      "published": ["chat"],
      "candidate": ["chat", "steer"],
      "declared_breaking": false,
      "expect": {"class": "additive", "next": "1.3.0", "removed": [], "added": ["steer"]}
    },
    {
      "name": "a declaration escalates at a stable major too",
      "current": "1.2.3",
      "published": ["chat"],
      "candidate": ["chat"],
      "declared_breaking": true,
      "expect": {"class": "breaking", "next": "2.0.0", "removed": [], "added": []}
    }
  ],
  "refuse": [
    {
      "name": "three slots with no numeric one would mean inventing an ordering",
      "current": "x.y.z",
      "published": ["chat"],
      "candidate": ["chat"],
      "expect": {"refusal": "not-numeric"}
    },
    {
      "name": "a mutable alias is a legal segment and still not a version",
      "current": "latest",
      "published": ["chat"],
      "candidate": ["chat"],
      "expect": {"refusal": "not-a-triple"}
    },
    {
      "name": "surrounding whitespace does not survive a path or a key",
      "current": "latest ",
      "published": ["chat"],
      "candidate": ["chat"],
      "expect": {"refusal": "not-canonical"}
    },
    {
      "name": "a character outside the canonical set is refused",
      "current": "latest+local",
      "published": ["chat"],
      "candidate": ["chat"],
      "expect": {"refusal": "not-canonical"}
    },
    {
      "name": "an empty published surface means a broken extractor, not a total removal",
      "current": "0.1.0",
      "published": [],
      "candidate": ["chat"],
      "expect": {"refusal": "empty-surface"}
    },
    {
      "name": "an empty candidate surface means the same",
      "current": "0.1.0",
      "published": ["chat"],
      "candidate": [],
      "expect": {"refusal": "empty-surface"}
    }
  ],
  "order": [
    {
      "name": "a numeric slot compares as a number",
      "older": "0.1.0",
      "newer": "0.1.1"
    },
    {
      "name": "minor outranks patch",
      "older": "0.1.1",
      "newer": "0.2.0"
    },
    {
      "name": "a coordinate that can never be bumped can still be ordered",
      "older": "latest.a",
      "newer": "latest.b"
    },
    {
      "name": "a numeric token sorts before a string token",
      "older": "0.1.0",
      "newer": "0.1.a"
    },
    {
      "name": "major outranks minor",
      "older": "1.3.0",
      "newer": "2.0.0"
    },
    {
      "name": "a stable version outranks every unstable one",
      "older": "0.2.0",
      "newer": "1.2.3"
    },
    {
      "name": "minor outranks patch at a stable major too",
      "older": "1.2.3",
      "newer": "1.3.0"
    }
  ],
  "order_equal": [
    {
      "name": "a version is not newer than itself",
      "left": "0.1.0",
      "right": "0.1.0"
    }
  ]
}
```

## What has actually been run against these cases

Twice, against two implementations.

The rule compiled into the Stado control plane reproduces **every classification
and every refusal**, driven by extracting the block with the reader described above
and feeding each side as a surface file. It cannot be checked against the ordering
cases at all, because no command it exposes compares two versions.

The Python port in this repository reproduces **all twenty-five cases**, ordering
included, through `conformance.py`.

### The suite was then attacked, and one section did not survive

A suite that has never failed proves nothing, so the port was deliberately broken
in three ways to see what the fixtures would catch:

| Sabotage | Disagreements reported |
| --- | --- |
| a breaking change advances major even while major is zero | five |
| a declared break is ignored instead of escalating | six |
| versions are compared as plain text instead of by slot | **none** |

The third result is a hole in these fixtures, not a success. Every ordering case
here happens to sort the same way under a naive character comparison, so a port
could compare version strings as text and pass.

Catching it needs one pair whose slots differ in digit count — a patch of nine
against a patch of ten, where the numeric order and the textual order disagree.
That pair cannot be written here yet: this workspace admits a numeric literal only
where a human named it, and no request in this project has named those two.

Until it is named, treat the ordering section as documentation of intent rather
than a check, and read `SPEC.md` for what the comparison must do. The same applies
to one classification case that is still missing: what an `internal` change
produces at a stable major.

### One divergence, found by running this

The surface contract in `SPEC.md` names the field `surface`. The implementation in
the control plane accepts only `commands`, and rejects a document shaped as
specified:

```
help output has no "commands" array
```

`surface` stays as written. A name in a surface may be an exported symbol, a route
or an event, and calling that set `commands` would only be correct for
command-line tools — the narrowest of the consumers this rule exists to serve.

So the divergence is recorded rather than smoothed over, and it means one precise
thing: **the control plane is not yet a port of this rule.** No alias will be
accepted to make it look like one, because a second accepted spelling is a second
way to answer the same question.
