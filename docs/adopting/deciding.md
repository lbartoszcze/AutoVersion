# Does the rule apply here?

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
| the manifest names a distribution coordinate somebody else owns — a fork or an imported mirror | the version is real but not ours; our surface can never diverge from the published one by any act performed here |
| one declared version that several shipped artifacts already share | the version exists and does not *discriminate*, so it cannot be what anybody selects |

The last row is the subtlest and the easiest to miss, because a manifest *does* state a
version and every mechanical check passes. Measured in this fleet: three artifacts published
to the release channel under three different coordinates, 54 MB and 198 MB and 198 MB — plainly
different builds — and all three declare the same `0.4.0`. The number cannot tell you which
one you are holding; the coordinate and the digest can. A gate over it would guard a value
that several shipped artifacts already share, which is not a contract. The diagnostic is
cheap and worth running whenever a channel publishes under something other than a version:
download two releases and read the manifest out of each.

The fork row hides a different failure mode, and it is the mirror image of a
sleeping gate. Wire a fork and the gate goes **permanently red**: the baseline recovers from
the upstream's latest release, our tree lags it, so the rule reports a change nobody here
made and demands a version nobody here can publish. Green is reachable only by bumping
(forbidden) or by fast-forwarding to the upstream's tree (their work, not ours). A gate that
can never pass certifies exactly as little as one that can never fail, and it trains people
to ignore a red build.

Measured on a real fork in this fleet, both ways, and both are wrong. Against the upstream's
latest release the rule reported `breaking` and demanded `2.0.0` — driven by two dozen names
the fork never removed and merely lacks, because it sits 130 commits behind. Against the
version the fork's own manifest declares, the rule reported `additive` and derived `1.1.0` —
**a version the upstream had already published, with entirely different content.** So the
coordinate the rule computes for our tree is occupied by a stranger's release. That is the
sentence to remember: not merely that the name is not ours, but that the number the rule
derives is already taken by somebody else's artifact.

And no marker spelling rescues it. Take the distribution name faithfully from the manifest,
as this document tells you to, and for a fork that name *resolves* — the registry serves it,
the content names it, every control passes — and the gate validates a stranger's project
under our repository's name. Establishing that a tree is somebody's mirror is therefore
worth more than any assertion built on top of it, and the fork flag is not the way: prove it
by commit-hash containment, since a hash Merkle-covers the whole tree and ancestry, and an
imported mirror is not flagged as a fork at all.

Whether anything consumes it *today* is not the test. A package with no users still has
a version somebody will pin tomorrow, and the ratchet costs nothing meanwhile. Private
visibility is not the test either: that governs who may fetch the artifact, not whether
its identity is a version.

A check over a product that fails the test passes vacuously and tells the next
maintainer the repository ships something it does not. Refusing is the deliverable, and
the evidence is the work.

### The unit of adoption is the distribution, not the repository

Ask the question once per **distribution**, not once per repository. Nothing in the rule is
repository-scoped: `decide` takes a current version and two surfaces, and knows nothing about
trees. So a repository that ships two distributions — say an npm package at the root and a
Python one in a subdirectory, each with its own version, each with its own consumers — has
**two** slots and needs two baselines and two checks, named for what they guard.

Refusing such a repository on "several independent versions and no canonical one" would be
the wrong reading of that row. It exists for a product whose *one* artifact cannot say which
version it is — five crates in a workspace that build one binary, ten values none of which is
inherited. Two genuinely separate artifacts, each carrying its own version, are not that
case; they are two products sharing a directory.

And when their surfaces have already drifted while both manifests still declare the same
number — which is what one repository here turned out to be doing, with four types exported
from the npm side that have no counterpart in the PyPI one, both manifests reading 0.1.0 —
that is not an argument for refusing. It is the defect a gate exists to catch, lying open.

That example arrives with a warning attached, because the first version of it was wrong. The
agent who found it first reported a wider drift — sink methods present on one side and absent
on the other — using a scanner that read from a class header to the end of the *file*, so
members of a later class were attributed to earlier ones. Read properly, those methods pair
exactly. It is precisely the failure this document warns about in extractors, committed in the
tool used to diagnose it, and it is why a surface reader must track class boundaries and fail
loudly rather than guess.

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
| installed only from a checkout | a `cargo publish`, a tag, or a first object under `stado://releases/<product>/<version>/<platform>/` |

Every refusal in this fleet is one workflow edit away from being wrong. Written this
way, it is revoked by observation instead of by somebody remembering.

One caution, and it is the mirror image of a bug this fleet hit in CI. A trigger that
consults git refs is born asleep: `actions/checkout@v4` fetches no tags, so "no tag
exists" is trivially true on a runner, and the refusal renews itself as *confirmed* at
exactly the moment the tag appears and it should have fallen. Worse than a false green,
because the empty listing agrees with the empty listing collected by hand, and two blind
reads look like independent corroboration.

There is a third trap in the same family, and it points the other way: **a fork shares the
upstream's object store, so tags you can see locally may not be yours.** Measured on a fork in
this fleet — after adding an upstream remote, `git tag --list` reports 23 tags while
`git ls-remote --tags origin` reports none. A probe reading local tags would file a baseline
under the upstream's tag as though this repository had been released, which is the
false-*positive* mirror of tag blindness. On another fork the single visible tag sat at
exactly the upstream tag's sha. So scope every tag question to `origin` explicitly, and
establish it before adding any upstream remote.

So establish "never tagged, never released" against the remote, never from a working
copy: `git ls-remote --tags origin`, the host's tags and releases API, and
`git rev-parse --is-shallow-repository` to know whether any local ref listing means
anything at all. Better still, prefer triggers that read no refs — a manifest field, or
the release channel's own answer about an object — which are immune to this whole class.

