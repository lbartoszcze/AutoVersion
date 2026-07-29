# Adopting the rule in a repository

A product supplies two things only it can know: its public surface, and the version it
declares. Everything else is the rule's, and the rule is not copied.

Three files land in the product. None of them contains the rule.

```
scripts/surface.py                    prints {"surface": ["name", ...]}
released-surface.json                 the surface of the version actually published
.github/workflows/version-check.yml   installs the rule, compares, refuses
```

## First decide whether the rule applies at all

**Consumer count is not the test, in either direction.**

Zero consumers does not excuse a product: a package with no users still has a version
somebody will pin tomorrow, and the ratchet costs nothing meanwhile. Real consumers do
not oblige one either: this fleet has a production service that four repositories send
live traffic through, and it still refuses, because every one of them selects a URL and
a bearer — never a version, a tag or a digest. The unit of consumption there is a
running deployment, so there is nothing for a version gate to guard.
The operative test is sharper than "is it installable", because installing from a git
URL selects a ref, not a version, in every ecosystem — so that question separates
nothing. Ask instead:

**Does the packaging stamp the version into a distributable artifact, and can a
consumer observe it?**

A Python distribution passes: the sdist and wheel filenames carry the version and
`pip show` reports it. A crate published to a registry passes. A binary published to a
channel under `/<version>/<platform>/` passes. Publication may not have happened yet —
adopt anyway, because the ratchet is cheapest to install before the first release
rather than after the first mistake.

These fail the test, and refusing is the correct outcome:

| Shape | Why there is no version to check |
| --- | --- |
| artifact pinned by content digest | the pin is a digest; no version segment exists, and none ever will |
| `releaseId: "source-tree"`, invoked by absolute path | the working tree *is* the release |
| `publish = false`, or a binary with no `--version` and no tag | nobody can observe the version, so nobody can select it |
| consumer holds only a service URL and a bearer | the unit of consumption is a running deployment |
| no packaging metadata at all | there is no artifact to carry a version |
| several independent versions and no canonical one | `--current` is undefined |
| built and installed only from a checkout — `cargo install --path .`, `pip install -e .` | the artifact is selected by path to a working tree, never by version |

Whether anything consumes it *today* is not the test. A package with no users still has
a version somebody will pin tomorrow, and the ratchet costs nothing meanwhile. Private
visibility is not the test either: that governs who may fetch the artifact, not whether
its identity is a version.

A check over a product that fails the test passes vacuously and tells the next
maintainer the repository ships something it does not. Refusing is the deliverable, and
the evidence is the work.

### A refusal needs a condition that revokes it

A wired repository leaves a frozen file in the tree, so a check can hang on it. A
refusal leaves nothing — which means when the reason stops holding, nobody finds out.
Refusals rot the same way a stale baseline does, only more quietly.

So record every refusal with the observation that would overturn it, phrased so someone
can check it without having been here:

| Refusal ground | What revokes it |
| --- | --- |
| channel is content-addressed | the publish workflow starts writing `/<version>/<platform>/`, or a tag appears |
| no packaging metadata | a manifest lands that names and versions the distribution |
| `publish = false`, no `--version`, no tag | any of those three changes |
| consumer holds only a URL | a consumer starts selecting a version, or the service reports one it can select |
| no canonical version | a single inherited version appears and the artifacts carry it |

Every refusal in this fleet is one workflow edit away from being wrong. Written this
way, it is revoked by observation instead of by somebody remembering.

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

### Do not let the baseline rot on a lower tier

Being honest about your tier is not the same as being on the right one. A tag or a
release can appear *after* the baseline was generated, and a `head:` baseline keeps
passing the bidirectional check while a better artifact sits unused — the marker is
truthful and the baseline is stale. Compare against the best reachable artifact in the
same step:

```sh
best="$(python3 scripts/baseline.py --stdout | jq -r '.source | split(" ") | first')"
if [ "${marker%%:*}" = head ]; then want="${best%%:*}"; have="${marker%%:*}"
else want="$best"; have="$marker"; fi
if [ "$want" != "$have" ]; then
  echo "::error::baseline is '$have' but '$want' is reachable now"
  false
fi
```

**Compare the whole marker on every tier except `head`, and the tier alone on `head`.**
The asymmetry is not fussiness. A `head:` marker carries a sha that moves with every
commit, so demanding full equality there is an infinite ratchet — a regenerated baseline
per commit, forever. Every other tier names its artifact exactly, by filename or by tag,
and those do not move; comparing only the prefix there passes a baseline that names the
right *kind* of artifact and the wrong one — an older sdist while a newer release
exists, or a tag outranked by a newer tag. Tier matches, marker is honest, and the gate
measures a superseded artifact forever.

On a registry tier, comparing *versions* is better still: it explains the failure —
"PyPI now serves 0.1.2" — instead of showing two filenames, and `autoversion order`
makes the ordering the rule's answer rather than the workflow's guess.

Print the candidate baseline to stdout. The committed `released-surface.json` must never
be rewritten by the check, and the regenerated *surface* must never reach the decision.
Recomputing both sides at check time is the one shape that genuinely cannot refuse
anything, which is precisely what a frozen committed file is not.

The motivating case is not hypothetical: one repository in this fleet published a
distribution whose packaged surface was empty. A stale baseline there measures every
later change against a surface that never existed.

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
