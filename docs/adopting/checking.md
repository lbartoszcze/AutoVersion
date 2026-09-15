# The check, and proving it can refuse

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

And do **not** fall back to `--break-system-packages` when the venv cannot be built. It is
tempting — it keeps the step green on an image without the `venv` module — and it is the one
branch the identity guard below cannot cover: the rule lands in the marked interpreter, so the
step has no way to attribute the binary that answers to its own install. Red and named beats
green through an unattributable binary. Let a missing `venv` module be reported and refuse.

One honest limit on all of this, and it belongs in the record rather than in a footnote: nobody
in this fleet reproduced the `externally-managed-environment` failure itself, because the
development machines run a Homebrew python that carries no marker. That half is taken from the
runner image. What *was* verified by execution is that the venv shape works, that the rule
answers from inside it, and that a stranger on `PATH` either loses the prepend or is named and
refused.

Then assert the rule actually answers before anything downstream claims a verdict — one
`autoversion --help`, or better a pair of known answers: identical surfaces must read
`internal`, and one removed name must read `breaking`. Proven load-bearing by putting a
sabotaged `autoversion` first on `PATH`: with the install step the gate passes, without it the
saboteur answers and the gate fails. Word the failure so it blames what is on `PATH`, not the
registry.

Better still, establish **which** `autoversion` is about to answer before believing it:

```sh
resolved="$(command -v autoversion || true)"
case "$resolved" in
  "$RUNNER_TEMP/rule/bin/"*) ;;
  *) echo "::error::autoversion resolves to '${resolved:-nothing}', outside the venv"; false ;;
esac
```

`--help` is satisfied by any executable that exits zero, so it certifies a stranger just as
happily as the rule. And the `export PATH` line above only repairs the *command-not-found*
branch: if a later edit drops it, `--help` goes back to exiting zero because something else
answered, and nobody sees it because the step is green. The identity check turns that fail-open
into a fail-closed — measured on a machine that has an ambient `autoversion` installed, dropping
the export makes the step refuse and *name* the interloper's path.

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
