# Keeping the baseline honest

A baseline that is never compared against the tier above it, or compared
without the tags to compare with, stops refusing anything.

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
[ -n "$best" ] && [ -n "$marker" ] &&
  [ "$best" != null ] && [ "$marker" != null ] || {
    echo "::error::a marker read empty, so this comparison would be vacuous"; false; }
if [ "${marker%%:*}" = head ]; then want="${best%%:*}"; have="${marker%%:*}"
else want="$best"; have="$marker"; fi
if [ "$want" != "$have" ]; then
  echo "::error::baseline is '$have' but '$want' is reachable now"
  false
fi
```

**The `!= null` half is the one that does the work, and `-n` alone does not.** Measured:
`jq -r '.source | split(" ") | first'` on an empty `.source` prints the literal string `null`
and exits zero — `"" | split(" ")` is an empty array, and `first` of that is null — so `-n` sees
four characters, calls it non-empty, and the comparison proceeds to find `null` equal to `null`
and pass. That is exactly the vacuous agreement the guard was added to prevent, sailing through
the guard.

And a correction to why this section exists. I wrote that a dead generator passes vacuously; on a
workflow with `set -euo pipefail` it does not, because `pipefail` carries the generator's failure
through the pipe. The dead generator was already refused — just silently, with no `::error::` to
diagnose it. The real teeth here are the vacuous comparison and the diagnosis, not the dead
generator. Run the generator into a file anyway: `pipefail` is a habit, not a guarantee, and the
status is worth testing where you can see it.

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

