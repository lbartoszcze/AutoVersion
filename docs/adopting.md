# Adopting the rule in a repository

A product supplies two things only it can know: its public surface, and the version it
declares. Everything else is the rule's, and the rule is not copied.

Three files land in the product. None of them contains the rule.

```
scripts/surface.py                    prints {"surface": ["name", ...]}
released-surface.json                 the surface of the version actually published
.github/workflows/version-check.yml   installs the rule, compares, refuses
```

## Read this page, then the rest only when it bites

This document is long because it was written by being wrong in public: nearly every section
below exists because a gate looked correct and was not. That length is itself a hazard — the
worst defect this fleet produced, a `pip install` that could never succeed on a runner, spread to
twenty repositories by being copied out of a page nobody finished. So the whole method is on this
page, and everything after it is the evidence and the traps.

**Does the rule apply?** Only if the packaging stamps a version into a distributable artifact and
a consumer can observe it. Consumer count is not the test in either direction. If it does not
apply, change **zero files** and record the ground plus the observation that would overturn it.
A refusal is a result.

**What to write.** Four files: an extractor that reads the surface statically and fails loudly, a
generator that recovers the published surface from the best reachable artifact, the frozen
`released-surface.json`, and a workflow that installs the rule and compares. Never copy the rule.
Never bump a version inside a check.

**The five things that will be wrong if you skip them.** Each one is invisible from a laptop and
each cost this fleet a day:

1. Install the rule into a **venv**, prepend it to `PATH` in-step, and assert `command -v
   autoversion` resolves inside that venv. A bare `pip install` cannot succeed on the current
   runner image, and a liveness probe alone is satisfied by any executable that exits zero.
2. Add a **tag fetch** after checkout if any step reads a ref: `actions/checkout@v4` fetches none,
   so a tier probe goes blind exactly where it matters.
3. Read absence from the **answer's content**, never from a client's exit status, and keep a
   positive control that runs through the same spelling *and* the same credential as the subject.
4. Prove the gate **can refuse**, in the same step that consumes the verdict. A fake rule
   answering a constant `internal` passes everything else.
5. Exercise all of it inside `git clone --depth 1 --no-tags`, and on the runner's own image if you
   can. Every defect in this document was found by simulating the runner, never by one.

**Then check your own work against the tooling**, which is in `wisent-ground-truth-docs` under
`tools/versioning/`: `versioning-gate-audit.py` reads a committed workflow for the defects above,
and `versioning-run-on-image.py` executes it on `ubuntu:24.04` with a runner-shaped environment.

Everything below is why. Read a section when its subject bites you.


The rest of this guide is beside this page:

- [Does the rule apply here?](adopting/deciding.md) — when it does not, and
  what a refusal needs before it can be revoked.
- [The surface and how to extract it](adopting/surface.md)
- [The baseline](adopting/baseline.md) — the controls it needs, how it rots,
  and the trap.
- [The check, and proving it can refuse](adopting/checking.md)
- [Keeping the baseline honest](adopting/keeping-honest.md) — the tier
  comparison, and the tags CI needs to make it.
