# Changelog

## Unreleased

- Every refusal of `autoversion` is a wisent-errors cause `src/errors`
  declares: the product (`autoversion`: a command or flag the invocation
  gets wrong), the commands (`autoversion.check`: a declared version the
  surface change does not make; `autoversion.decide` and
  `autoversion.surface` refuse only through what they use) and the layers
  they run (`autoversion.document`, the surface documents `decide` reads;
  `autoversion.python`, a package's `__all__` or module bindings;
  `autoversion.rule`, one cause per refusal kind of docs/FIXTURES.md). The
  binary prints the sentence it always printed, `refused: …` or
  `autoversion: error: …` with the usage, then the envelope on one JSON line,
  and exits as before (2 for the invocation, 1 for every other); stdout is
  unchanged. `autoversion::surfaces::SurfaceError` names which reading it
  refused (`refusal`, a `SurfaceRefusal`) beside its `message`. The
  envelope's dependency is the `cli` feature, the binary's, on by default;
  `default-features = false` still takes the rule with serde_json alone.
  `src/errors/check.rs` holds the catalogue to the standard, the tests read
  the refusal before the envelope and check the cause it names, and the
  README lists every cause under Refusals.
