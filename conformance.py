"""Check a port against FIXTURES.md.

The fixtures are the contract between ports; the compiler keeps nothing honest
across languages, so this runner does. It reads the cases the way the fixtures
document describes — the first fenced block — so the file stays readable prose
with one machine-readable contract inside it.

Exits non-zero on the first disagreement, listing every one it found.
"""

from __future__ import annotations

import json
import pathlib
import sys

from autoversion import rule

FENCE = "```"


def load_cases(path: pathlib.Path) -> dict:
    body = path.read_text()
    block = body.split(FENCE)[int(True)]
    if block.startswith("json"):
        block = block[len("json") :]
    return json.loads(block)


def check(cases: dict) -> list:
    failures = []

    for case in cases["classify"]:
        try:
            got = rule.decide(
                case["current"],
                case["published"],
                case["candidate"],
                case["declared_breaking"],
            )
        except rule.RuleError as error:
            failures.append(f'classify {case["name"]!r}: refused with {error.refusal}')
            continue
        want = case["expect"]
        for field in ("change", "next", "removed", "added"):
            if got[field] != want[field if field != "change" else "class"]:
                failures.append(
                    f'classify {case["name"]!r}: {field} was {got[field]!r}, '
                    f'expected {want[field if field != "change" else "class"]!r}'
                )

    for case in cases["refuse"]:
        try:
            rule.decide(case["current"], case["published"], case["candidate"], False)
        except rule.RuleError as error:
            if error.refusal != case["expect"]["refusal"]:
                failures.append(
                    f'refuse {case["name"]!r}: refused with {error.refusal!r}, '
                    f'expected {case["expect"]["refusal"]!r}'
                )
        else:
            failures.append(f'refuse {case["name"]!r}: was accepted')

    for case in cases["order"]:
        if not rule.version_newer(case["older"], case["newer"]):
            failures.append(
                f'order {case["name"]!r}: {case["newer"]!r} did not sort after '
                f'{case["older"]!r}'
            )
        if rule.version_newer(case["newer"], case["older"]):
            failures.append(
                f'order {case["name"]!r}: the comparison is not strict in one direction'
            )

    for case in cases["order_equal"]:
        if rule.version_newer(case["left"], case["right"]):
            failures.append(f'order_equal {case["name"]!r}: a version outranked itself')

    return failures


def main() -> int:
    here = pathlib.Path(__file__).resolve().parent
    cases = load_cases(here / "FIXTURES.md")
    counts = {name: len(section) for name, section in cases.items()}
    failures = check(cases)

    total = sum(counts.values())
    for name, count in counts.items():
        print(f"  {name}: {count}")
    if failures:
        print(f"\n{len(failures)} disagreement(s) out of {total} case(s):")
        for failure in failures:
            print(f"  {failure}")
        return int(True)
    print(f"\nall {total} cases reproduced")
    return int(False)


if __name__ == "__main__":
    sys.exit(main())
