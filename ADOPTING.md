# Adopting the rule in a repository

A product supplies two things only it can know: its public surface, and the version it
declares. Everything else is the rule's, and the rule is not copied.

Three files land in the product. None of them contains the rule.

```
scripts/surface.py                    prints {"surface": ["name", ...]}
released-surface.json                 the surface of the version actually published
.github/workflows/version-check.yml   installs the rule, compares, refuses
```

## First decide whether a version is consumed at all

A version is consumed when somebody outside the repository selects it: a package
release, an artifact under `stado://releases/<product>/<version>/...`, a store build
number, a container tag, or a sibling repository pinning it.

If nothing selects a version — a paper, a schema, a landing page, a research script —
**do not adopt.** A check over a product that distributes nothing passes vacuously and
tells the next maintainer the repository ships something it does not. Refusing is the
correct outcome, and saying why is the deliverable.

## Choosing the surface

The surface is the set of names a user would notice disappearing. It is not always the
package's symbols:

| Shape of product | What callers actually hold |
| --- | --- |
| library | entries of `__all__`, and the public methods of exported classes |
| registry or manifest package | the identifiers it registers — task, evaluator, rule names |
| command-line tool | the commands its help *advertises*; a command that dispatches but is unlisted is private |
| service or gateway | its routes, or the tool names it exposes |
| scanner or linter | its rule identifiers, plus anything that silently changes what it looks at |
| distribution with entry points | console script names, since a rename breaks a script that ran yesterday |

Two rules of thumb earned the hard way. **Include a set whose removal your surface would
otherwise call internal** — a chart library whose exported class names never change while
`plot()` is deleted is not protected by counting class names. **Exclude what a product is
expected to improve** — a scanner's regexes change as it gets better at finding things;
the rule identifier is the promise, the pattern is an implementation.

Say in the extractor's docstring why the set you chose is the contract. The next person
will disagree with something, and they should be arguing with a stated reason.

## Extracting it

Read the surface statically. Never import the product.

Importing runs side effects and demands the product's dependencies, and a release
decision must not require a machine that has `torch` installed. Reading statically also
means the same extractor runs against an unpacked published artifact, which is how a
baseline is recovered rather than assumed.

**A module that does not parse must fail loudly.** Skipping it reports a smaller surface,
and the rule reads a smaller surface as removed capability — a false `breaking` verdict
for an unrelated syntax error. The surface is unknown there, not shrunk. Keep a
`--tolerant` mode only for recovering an already-published artifact, and have it report
every module it skipped.

## The baseline

`released-surface.json` describes the version **actually published**. Its `"source"`
field starts with a marker naming which artifact it was recovered from; everything
after the first space is prose for humans.

| Marker | Recovered from | Claims a registry |
| --- | --- | --- |
| `pypi-sdist:<filename>` | a published sdist | yes |
| `pypi-wheel:<filename>` | a published pure-Python wheel | yes |
| `stado:<object path>` | an artifact in the release channel | yes |
| `git-archive:<tag>` | a tag, reproduced with `git archive` | no |
| `head:<full sha>` | the working revision — last resort | no |

Preference runs down that table. Pick the best tier that actually exists for you, never
a lower one because a higher one was inconvenient. Prefer a tag over HEAD even when
nothing reached a package index: a tag is something somebody installed. If a tag
disagrees with the version inside it, use the tag that really contains that version and
report the mismatch.

Read the marker as a token, never by matching prose:

```sh
marker="$(jq -r '.source | split(" ") | first' released-surface.json)"
```

Use `first`, not a `[0]` subscript: this workspace refuses bare numeric literals in
files, so the subscript form cannot be written into a workflow at all.

Trust a tag only when the tree it points at declares the version the tag name claims.
A tag that disagrees is reported and skipped, never filed under the version it claims —
at least one repository in this fleet has a tag pointing at a tree that still declared
the previous version, and believing the name would have measured everything afterwards
against the wrong artifact.

Guard it in both directions, each family against its own registry, because a baseline
nobody can install measures every later comparison against nothing:

- a marker that claims a registry → that exact version must be served there;
- a marker that claims none → the registry must not serve this project at all. If it
  does, the baseline is dodging a real release and the check must refuse.

Couple the two files through a named constant in your generator, not through prose that
drifts.

### The trap

**Resolve the latest published version from the registry, not the version the manifest
declares.** The moment someone bumps ahead of a release, looking up the declared version
returns nothing, a naive generator degrades to `head:<sha>`, and it throws away the real
published baseline — after which every comparison is measured against the wrong
artifact, quietly. Ask the registry what the newest published version is, then recover
that version's surface.

One consequence worth planning for: a wheel contains no manifest. If console scripts are
part of your contract, read them from `<dist>-<version>.dist-info/entry_points.txt`
under `[console_scripts]` rather than from `pyproject.toml`.

## The check

```sh
pip install "git+https://github.com/lbartoszcze/AutoVersion@v0.1.0"
autoversion decide --current "$released" \
  --published-surface released-surface.json \
  --candidate-surface "$candidate" --json
```

Then compare the version the product declares with the version the rule derived:

- declared equals released and the change is `internal` → nothing to release, pass;
- declared equals released and the change is not `internal` → refuse, naming the
  required version;
- declared differs from the required version → refuse, naming both.

Never bump a version inside the check. Deriving the number is mechanical; deciding to
release is deliberate, and a published coordinate must resolve to a revision already
pushed.

## Prove the gate can refuse

A gate that has never refused is decoration. Before you are done, run the comparison
against a surface with one name removed and one added, and confirm they yield `breaking`
and `additive`. Keep that as a script if it helps the next person believe it.
