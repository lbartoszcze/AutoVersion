# The baseline


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

And **the same access, not just the same spelling.** A control needing wider credentials than
the subject is not a control; it is a second subject. Measured in this fleet: one gate's positive
control queried a *different, private* repository, and `secrets.GITHUB_TOKEN` on a runner is
scoped to the repository being built — so on every run it would receive a clean 404 for a
repository that demonstrably has releases, conclude its own verdict was worthless, and refuse.
Permanently red, and red for a reason nobody would guess from the message. Prefer a control on
the **same repository** you are already asking about; failing that, one that is explicitly
public.

There is a companion mistake on the reading side, and an agent here caught themselves in it:
they read a workflow out of a clone taken at the start of their work and ran their arms against
a clone taken later, after the branch had moved, then drew one conclusion from both. The read
and the run were different objects. Pin the revision you are asserting about and check that both
halves of your evidence come from it.

Class 2 also has a shape worth knowing, because one registry makes it indistinguishable by
structure alone. PyPI states absence unambiguously (`{"message": "Not Found"}`), but
crates.io returns the *same* envelope for "no such crate" and for a policy refusal —
`{"errors": [{"detail": …}]}` either way, and it refuses requests with no `User-Agent` by
default, which is the default state of many CI images. So match the detail *string*
(`does not exist`), never the presence of `errors`, and send a User-Agent.

Two smaller consequences, and the first one carries a correction worth more than the
observation it replaces. Read the field that answers your question: on a present object
`state` is authoritative while `version` is empty and `detail` carries an unrelated
diagnostic, so a check branching on `detail` inverts its own answer.

And this document previously said a listing subcommand was defective regardless of spelling,
because `ls` reported "0 of 0" for six objects that demonstrably existed. **That was measured
against a stale binary.** The installed control plane predated its own source by eighty
minutes, and a commit in between — *"resolve object paths in one place"* — had already fixed
it. Built from the source the fleet was reading, the same command answers `6 of 6`. The bare
path still lists nothing, and that is correct rather than broken: without the namespace root a
bare path is a raw backend key that genuinely does not exist, exactly as it is for `stat`.

So the durable lesson is not about that tool. Six agents and I diagnosed a defect that had
already been repaired, because every one of us compared the tool's *answer* to the tool's
*source* without once checking that the binary was built from it. That is the silent-argument
error one level up: the probe ran, succeeded, and answered truthfully about a different build
than the one under discussion. Before concluding that a tool is broken, establish its
provenance — compare the binary's build time to the last change of the source you read, or
build from that source and ask again.

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

