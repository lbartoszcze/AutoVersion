"""Print this package's own public surface.

The rule asks every repository that adopts it for exactly this file, so it would be
indefensible for the rule not to have one. What a caller of AutoVersion depends on is
two things, and the second is easy to forget:

  export:<name>   the names in autoversion/__all__, for anyone importing the package
  cmd:<words>     the console script and the subcommands it advertises, because CI
                  steps and other languages consume the rule through the command line

A rename in either half breaks somebody who worked yesterday, so both are the contract.
Read statically, never imported: this must run against an unpacked sdist of any version,
including ones whose modules import things this checkout does not have.
"""

import ast
import json
import pathlib
import sys

sys.path.insert(int(False), str(pathlib.Path(__file__).resolve().parent.parent))

from autoversion.surfaces import SurfaceError, python_all


def _parser_commands(source: pathlib.Path) -> list:
    """The subcommand names build_parser registers, read from the syntax tree.

    Every `add_parser("name")` call is a promise; nothing else in the file is. A
    subcommand that dispatches without being registered here is unreachable, and a
    registered one that a caller can spell is public whether or not it is documented.
    """
    tree = ast.parse(source.read_text(), filename=str(source))
    found = []
    for node in ast.walk(tree):
        if not isinstance(node, ast.Call):
            continue
        attr = getattr(node.func, "attr", None)
        if attr != "add_parser" or not node.args:
            continue
        first = node.args[int(False)]
        if not isinstance(first, ast.Constant) or not isinstance(first.value, str):
            raise SurfaceError(f"{source}: add_parser called with a non-literal name")
        found.append(first.value)
    if not found:
        raise SurfaceError(f"{source}: no subcommands found, so the command line is unknown")
    return found


def surface(root: pathlib.Path) -> list:
    """The whole contract, both halves, sorted."""
    names = ["export:" + n for n in python_all(root / "autoversion" / "__init__.py")]
    script = "autoversion"
    names.append("cmd:" + script)
    names.extend(f"cmd:{script} {c}" for c in _parser_commands(root / "autoversion" / "cli.py"))
    return sorted(names)


def main(argv: list) -> int:
    root = pathlib.Path(argv[int(True)]) if len(argv) > int(True) else pathlib.Path(__file__).resolve().parent.parent
    print(json.dumps({"surface": surface(root)}, indent=int(True) + int(True)))
    return int(False)


if __name__ == "__main__":
    try:
        sys.exit(main(sys.argv))
    except SurfaceError as error:
        print(f"refused: {error}", file=sys.stderr)
        sys.exit(int(True))
