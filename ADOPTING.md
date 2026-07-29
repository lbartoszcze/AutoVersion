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

**And "fails loudly" is harder than it looks in a hand-written scanner.** Tracking brace depth
and erroring on imbalance is necessary and *not* sufficient: a mangled or truncated class
header simply stops matching. Nothing is unbalanced, nothing errors, the class contributes no
members, and the surface comes out shorter — which the rule reads as a breaking removal nobody
made. The corruption arm that is supposed to catch this misses it easily: deleting a file's
first `}` usually mangles an import, and a scanner reads straight through that to the correct
answer, reporting a false clean.

What closes it is resolution rather than syntax. Take every name the entry point re-exports and
resolve it back to a declaration in the module that is supposed to hold it; an unresolvable name
or a missing module is a refusal. Then a header that stopped matching cannot be silent, because
the names that depended on it no longer resolve. Exercise it with three arms, not one:
unbalanced, balanced-but-unmatchable, and missing module.

## The baseline

`released-surface.json` describes the version **actually published**. Its `"source"`
field starts with a marker naming which artifact it was recovered from; everything
after the first space is prose for humans.

| Marker | Recovered from | Claims a registry |
| --- | --- | --- |
| `pypi-sdist:<filename>` | a published sdist | yes |
| `pypi-wheel:<filename>` | a published pure-Python wheel | yes |
| `npm-tarball:<registry path>` | a published npm tarball, as the registry addresses it — `express/-/express-4.22.2.tgz`, or `@types/express/-/express-5.0.6.tgz` for a scoped one | yes |
| `crates-io:<filename>` | a published crate, `<name>-<version>.crate` | yes |
| `stado:<object path>` | an artifact in the release channel | yes |
| `gh-release:<tag>` | an asset on a GitHub Release | yes |
| `git-archive:<tag>` | a tag, reproduced with `git archive` | no |
| `head:<full sha>` | the working revision — last resort | no |

Preference runs down that table. Pick the best tier that actually exists for you, never
a lower one because a higher one was inconvenient. Prefer a tag over HEAD even when
nothing reached a package index: a tag is something somebody installed. If a tag
disagrees with the version inside it, use the tag that really contains that version and
report the mismatch.

Take the npm path from `.versions[<v>].dist.tarball` and never assemble it. Two traps sit
in that one row, both measured. The registry serves a **scoped** package's tarball under the
**unscoped** filename — `@types/node` yields `node-26.1.2.tgz` — so a marker built from the
scope refuses forever against a healthy artifact. And the basename alone is **not unique**:
`express` and `@types/express` both serve `express-<version>.tgz`, two different packages
with different owners and different contracts. A marker exists to identify the artifact the
baseline came from, and whole-marker comparison depends on it, so carry the registry path.
That is this document's own complaint about generic answers, turned on our own vocabulary.

And keep the two names apart: the **filename** drops the scope, but the **lookup** must be
the full scoped name read from `package.json`. Stripping the scope to query is not a
harmless shortcut — `@types/node` and `node` are both real packages that answer, so a
reverse assertion on the bare name does not fail closed, it validates a stranger's project.
Measured alongside it: a lookup with an unencoded slash and no scope sigil returns HTTP 405,
not 404, which a two-state check reads as proven absence and a three-state one calls
unproven.

### Every negative needs a positive control

This is the one rule the wave earned the hard way, and it generalises every bug found in
it: **absence inferred from a probe that may not have run is not evidence.** A probe that
silently did nothing and a probe that ran and found nothing return the same answer, and
the wrong reading is always the passing one.

So beside every assertion of absence, run the same probe against something you know is
present, and fail if the control comes back empty. Four instances, all real:

- **The reverse assertion is fail-open.** "This project is on no registry" is normally
  written as `curl -sSf … || echo not published`. But curl also fails on no egress, DNS
  failure, or the registry being down, and at that level those are indistinguishable from
  the answer you meant to read. So the step concludes "nothing is published, the baseline
  is honest" *precisely when it made no successful request*. Worse than a stale baseline:
  that one needed a tag to appear before it mattered, this one is wrong on every hiccup.

  A positive control on a project you know is published is the obvious repair, and it is
  **not enough**. It proves the index is reachable, not that *your* project's lookup
  answered — a rate-limit page, an error page or a permission refusal on that lookup fails
  exactly like not-found and still reads as absence. A 403 is the sharp case: a successful
  request, a real response, a control that passes cleanly, and an answer that means
  nothing.

  So **demand testimony from the answer's content.** Three outcomes instead of two — named,
  stated-absent, unproven:

  ```sh
  answer="$(curl -s "https://pypi.org/pypi/$project/json" || true)"
  if printf '%s' "$answer" | jq -e '.info.name' >/dev/null; then
    echo "::error::$project IS published, so a no-registry baseline is a lie"; false
  elif ! printf '%s' "$answer" |
       jq -e '(.message // "") | ascii_downcase | contains("not found")' >/dev/null; then
    echo "::error::the index did not answer, so the absence of $project is unproven"; false
  fi
  ```

  The `|| true` is safe *only* because both passing branches demand specific content and
  the fallthrough refuses. Never lift it into a check that treats an empty answer as fine.
  Reading the status code would be equivalent, but `%{http_code}` and `= 404` are bare
  numerals that this workspace refuses to write, so content is the writable spelling.

  **Keep the positive control anyway, and point it at the positive branch.** Content covers
  transport silence, so the control looks redundant — it is not, because "unproven" is also
  what a *broken expression* produces. A `jq` path that can no longer recognise a published
  project refuses forever while the registry answers perfectly, and the operator blames the
  registry. That is the fail-*closed* twin of this bug: equally invisible locally, equally
  wrong. So assert that the check can still see something that certainly exists, and say
  which side is broken:

  ```sh
  curl -s "https://pypi.org/pypi/pip/json" | jq -e '.info.name' >/dev/null || {
    echo "::error::this step cannot recognise a project PyPI definitely serves,"\
         "so its verdict is meaningless"; false; }
  ```

  One more asymmetry decides how much that control has to carry: **does the registry name
  your subject back?** crates.io echoes it (``crate `x` does not exist``) and is therefore
  self-validating. PyPI and npm answer generically, so a lookup of the *wrong or empty*
  name reads as proven absence — and the name usually comes from parsing a manifest, which
  yields empty the moment `[project] name` moves or turns dynamic. Where the registry does
  not name the subject, assert the input, or control the request shape, or both.

  Do not branch on a specific exit code either: on `.info.name` over a body with no
  `.info`, `jq` exits 1 while `jaq` exits 5. Both are non-zero, so a truthiness test is
  portable and an exit-code comparison is not — and on this workstation the bare `jq` on
  PATH is `jaq`, so "I tested with jq" may not mean what you think.

  And if your check mixes shell and Python, a stubbed `curl` does not exercise the Python
  path: it keeps real network and *looks* verified. Break both transports, or your proof
  of fail-closed behaviour is itself a false positive of the class you are hunting.

- **Ask the right store.** A channel probe that queries the wrong surface reports absence
  for objects that demonstrably exist, on every invocation, with a zero exit. A reverse
  assertion written that way passes unconditionally forever and certifies nothing; a
  forward one fails forever and goes red the day the outage lifts, when everyone will
  assume the surface changed. The control — a product you know is published — is what
  catches it, because the subject's answer looks perfect either way.

  Better still, **enumerate the namespace instead of filtering it.** A listing of the
  whole channel shows your product absent from a complete inventory; a query filtered by
  a product name you guessed shows nothing when the name is wrong, which is the same
  answer for a different reason. Enumeration removes that failure mode rather than
  controlling for it.

  Read registry absence from the HTTP **status code**, never from the client's exit
  status: an unreachable host gives you no code at all, which is visibly different from a
  404, whereas both give you a non-zero exit.

  This is a third failure class, distinct from silence. A positive control catches a probe
  that could not run; it does not catch one that ran, succeeded, and answered about
  something else. The request completes, the answer is unambiguous, and it is false —
  which is worse than a timeout, because a tool that says "the store answered, it is not
  there" is now vouching for a wrong answer. Only a control on a subject you know
  independently to exist can catch that.

  Two concrete lessons from this toolchain, both measured. The spelling of the argument
  decides which surface you hit: a full `stado://` URI resolves in the product namespace
  (`present`, 1168544 bytes) while the same object as a bare path reads as a queue key and
  answers `absent`. And prefer a probe that reports three states — present, absent,
  unreachable — over a listing, which collapses all three into one silence.

  Best of all, **prefer a fact read out of the repository over an absence read off a
  service.** "This manifest declares no `description` and no `license`, which the registry
  requires" is a property of the tree in front of you. "The registry does not serve it" is
  a property of a conversation that may not have happened. That example is real and
  mechanical: crates.io rejects a publish server-side, HTTP 400, when `description` is
  empty, or when `license` *and* `license-file` are both empty
  (`rust-lang/crates.io`, `src/controllers/krate/publish.rs`). So "this crate is not
  publishable as it stands" is decidable by reading `Cargo.toml`, with no request to
  anything — while `cargo publish --dry-run` merely *warns* and exits zero, which is why
  the fact is invisible from a laptop and why a locally produced `<name>-<version>.crate`
  proves no tier at all.

- **A search that found nothing may not have looked.** Before reporting "no repository
  pins this", run a pattern that must match through the same paths. This wave caught a
  directory renamed mid-task that way: the sweep reported a clean negative, and the
  control revealed the path had gone missing rather than been read.

- **The runner is not your laptop.** Our clones have exactly the refs and history a
  runner lacks, a bias in one direction, so exercise the committed step bodies inside
  `git clone --depth 1 --no-tags`. It is the only local technique in this fleet that can
  falsify a claim about CI.

Those instances are three distinct failure classes, and the third is the one that cost
this fleet the most rounds of correction:

1. **Silence.** The probe could not run. A positive control catches it.
2. **Authorised silence.** The probe ran and was refused — a 403, a rate-limit page. The
   control passes cleanly and the answer means nothing. Reading the answer's *content*
   catches it; the control does not.
3. **Silent argument error.** The probe ran, succeeded, and answered truthfully — about a
   different object than the author meant. Neither a control nor content-reading catches
   this, because both are working correctly. The request is fine; the question was wrong.

Only one discipline closes the third: run the control on a subject you know independently
to exist, **through the exact same spelling as the subject**. In this toolchain a full
`stado://` URI resolves in the product namespace while the same object as a bare path
answers `absent` — a true answer about a queue key. Measured, on an object that exists:
the URI gives `present` with a size, the bare path gives `absent`. That cost three agents
two mistakes each, in both directions, before anyone ran the control through both
spellings.

Class 2 also has a shape worth knowing, because one registry makes it indistinguishable by
structure alone. PyPI states absence unambiguously (`{"message": "Not Found"}`), but
crates.io returns the *same* envelope for "no such crate" and for a policy refusal —
`{"errors": [{"detail": …}]}` either way, and it refuses requests with no `User-Agent` by
default, which is the default state of many CI images. So match the detail *string*
(`does not exist`), never the presence of `errors`, and send a User-Agent.

Two smaller consequences. A listing subcommand may be defective regardless of spelling —
here `ls` reports "0 of 0" for six objects that demonstrably exist, both ways, so an
assertion built on it cannot fail and therefore certifies nothing. And read the field that
answers your question: on a present object `state` is authoritative while `version` is
empty and `detail` carries an unrelated diagnostic, so a check branching on `detail`
inverts its own answer.

One workspace rule ties three surprises together, so learn it once rather than three
times: **any index or key that would need a lone number has a word form.** `jq '… | first'`
not `[0]`, `.errors | first | .detail` not `.errors[0].detail`, and a fetch step instead of
`fetch-depth: 0`. The numeric-literal policy eats the bare digit in every one of them.

And keep the epistemics honest in what you write down. The remote can tell you a project
has no tags and no releases **now**; it cannot tell you it never had any, because a
deleted ref leaves no trace once the event window expires. Write "not released now,
established at the remote" and not "never released" — unless your ground is packaging, in
which case the distinction is moot: with no manifest, no artifact could ever have carried
a version, whether or not a ref once existed.

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
python3 scripts/baseline.py --stdout > "$RUNNER_TEMP/best.json" || {
  echo "::error::the baseline generator failed, so the best reachable tier is unknown"; false; }
best="$(jq -r '.source | split(" ") | first' "$RUNNER_TEMP/best.json")"
[ -n "$best" ] && [ -n "$marker" ] || {
  echo "::error::a marker read empty, so this comparison would be vacuous"; false; }
if [ "${marker%%:*}" = head ]; then want="${best%%:*}"; have="${marker%%:*}"
else want="$best"; have="$marker"; fi
if [ "$want" != "$have" ]; then
  echo "::error::baseline is '$have' but '$want' is reachable now"
  false
fi
```

The first four lines are not ceremony, and the shorter form I published first was wrong. In a
command substitution containing a pipeline the generator's exit status is **discarded**, and
`set -e` does not reach inside one. So a dead generator yields an empty `best`, and the step
blames the wrong thing — or, if the committed marker ever also reads empty, compares `""`
against `""` and **passes vacuously**. Run the generator into a file, test its status
explicitly, and assert both markers are non-empty before comparing them.

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

### The tier check is asleep in CI unless you fetch tags

`actions/checkout@v4` fetches one commit and no tags. So `git tag --list` is empty on the
runner whatever the remote holds, the tier probe concludes `head:` is still the best
artifact available, and it passes — blind to the exact tag it exists to notice. Green on
a laptop whose clone has tags, decorative on the runner. That is the worst shape a gate
can take.

```yaml
- name: Make tags and history visible
  run: git fetch --force --tags --unshallow || git fetch --force --tags
```

Unshallowing matters on its own: a shallow clone lacks the tag's tree, so `git archive
<tag>` fails even once the tag is visible. `--unshallow` errors on an already complete
repository, hence the fallback. `fetch-depth: 0` on the checkout step is the usual
spelling, but this workspace refuses the bare numeric literal, so the fetch step is the
writable form.

The deciding question is not which tier you are on. It is **does any step read a git ref
or git history.** Add the fetch step if a step invokes the generator, resolves a tag, or
runs `git archive`; skip it if every fact the check consults arrives over HTTP or from an
`ast` read of the checked-out tree. Registry tier correlates with not needing it, but the
correlation is not the invariant — a registry-tier repository that calls the generator
from CI is still reading refs it cannot see. Getting this wrong in the safe direction
costs a wasted second; getting it wrong the other way leaves a sleeping gate.

A related trap when recovering a baseline from a tag: `git archive <tag> | tar -x` gives
you the tree, but invoking the language's tooling inside it — `cargo metadata`, a build,
an import — reaches the network or a lockfile the tag never shipped, and the baseline
quietly becomes a property of the runner's cache instead of the artifact. Read the
manifest and sources statically out of the archive. It is the same discipline as never
importing a package to read its surface, one ecosystem over.

### The trap

**Resolve the latest published version from the registry, not the version the manifest
declares.** The moment someone bumps ahead of a release, looking up the declared version
returns nothing, a naive generator degrades to `head:<sha>`, and it throws away the real
published baseline — after which every comparison is measured against the wrong
artifact, quietly. Ask the registry what the newest published version is, then recover
that version's surface.

**Unless the name is shared, in which case bind the tier by digest.** That rule assumes the
distribution name is yours, and one repository here proved what happens when it is not. Its
`setup.py` claims a name whose later releases were published from a *different* repository in
the same fleet — 50-odd of them. Follow "latest published" literally and the generator adopts
another product's surface: measured, that release's sdist yields ~999 names with **zero**
overlap with this tree's 76, jamming the gate on a permanent meaningless `breaking`, while the
same release's *wheel* yields **zero** names, after which the gate can never read `breaking`
again. One release, two artifacts, disagreeing with each other — and the dangerous half is the
one that passes.

Name plus version cannot catch that, because both are correct; it is the silent-argument-error
class wearing a well-formed answer. Only the digest can. So resolve the newest published
release **whose artifact digest this tree vouches for** — from a built distribution under
`dist/`, or from the digest recorded in the committed baseline's own prose — and have the check
re-verify it. A digest cannot name the wrong object. If the evidence disappears the tier
honestly falls and the check refuses, which is the correct failure.

One consequence worth planning for: a wheel contains no manifest. If console scripts are
part of your contract, read them from `<dist>-<version>.dist-info/entry_points.txt`
under `[console_scripts]` rather than from `pyproject.toml`.

## The check

```sh
: "${RUNNER_TEMP:=$(mktemp -d)}"
: "${GITHUB_PATH:=$(mktemp)}"
python3 -m venv "$RUNNER_TEMP/rule"
"$RUNNER_TEMP/rule/bin/python" -m pip install --quiet \
  "git+https://github.com/lbartoszcze/AutoVersion@v0.1.0"
echo "$RUNNER_TEMP/rule/bin" >> "$GITHUB_PATH"
export PATH="$RUNNER_TEMP/rule/bin:$PATH"
autoversion decide --current "$released" \
  --published-surface released-surface.json \
  --candidate-surface "$candidate" --json
```

Default **both** runner variables, not just the temp directory. Under `set -eu` the line
`>> "$GITHUB_PATH"` dies with `GITHUB_PATH: unbound variable` when you run the step body
locally — which is exactly what you are told to do — so the venv builds, the rule installs, and
the step still fails. And export `PATH` as well as appending to `GITHUB_PATH`: the append only
affects *later* steps, so without the export the rest of this step cannot see the rule it just
installed.

**Install into a virtual environment, not the system interpreter.** A bare
`pip install git+…` was what this document said first, and it is a permanently red gate on a
current runner: `ubuntu-latest` now resolves to Ubuntu 24.04, whose system `python3` carries
the externally-managed marker, so the install fails with
`error: externally-managed-environment` before any check runs. A gate that can never pass is
worth what one that can never fail is worth. The venv sidesteps the marker without
`--break-system-packages`, and putting its `bin` on `GITHUB_PATH` keeps every later step's
`autoversion` call unchanged.

Then assert the rule actually answers before anything downstream claims a verdict — one
`autoversion --help`, or better a pair of known answers: identical surfaces must read
`internal`, and one removed name must read `breaking`. Proven load-bearing by putting a
sabotaged `autoversion` first on `PATH`: with the install step the gate passes, without it the
saboteur answers and the gate fails. Word the failure so it blames what is on `PATH`, not the
registry.

One snag if you write that control: an **empty** candidate surface will not serve as the
`breaking` case, because the rule refuses it outright — an empty surface is far more likely to
mean a broken extractor than a deleted API, which is the whole reason it refuses. Give the
control a non-empty candidate with one name missing.

And one more thing the control has to do, which is not obvious. Asserting the rule *answers* is
not enough: a saboteur returning a constant `internal` sails through the gate steps themselves,
because when declared equals released, `internal` is the "nothing to release" branch — the
passing one. The only step that catches it is one demanding the gate can still **refuse**: feed
it a surface with a name removed and require a non-zero exit. A workflow without a refusal
self-test accepts a fabricated verdict silently.

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
