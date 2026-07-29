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

`released-surface.json` describes the version **actually published**, in this order of
preference:

1. the published sdist,
2. the published wheel, when the package is pure Python,
3. the artifact in the release channel,
4. a git tag, recovered with `git archive`,
5. HEAD — last resort, and `"source"` must say so plainly.

Prefer a tag over HEAD even when nothing reached a package index: a tag is something
somebody installed. If a tag disagrees with the version inside it, use the tag that
actually contains that version and report the mismatch.

Guard the baseline in both directions, because a baseline nobody can install measures
every later comparison against nothing:

- if the registry serves that version, the baseline must declare it came from that
  artifact;
- if the registry does not, the baseline must not claim publication.

Couple the two files through a named marker in `"source"`, not through prose that drifts.

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
