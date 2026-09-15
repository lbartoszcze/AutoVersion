# The surface, and how to extract it

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

