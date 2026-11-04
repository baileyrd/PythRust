"""M6d's driver check: fclone_fast finds the groups fclone_lite.py finds.

Usage: python3 ci/fclone_driver.py FAST ORACLE

FAST is the fclone_fast program `pythrust build` made from fclone-lite's
manifest and lock, outside the repository; ORACLE is fclone_lite.py, run
under this Python. Both scan the same small tree: two duplicated
payloads of one size, an empty pair and a unique file. Paths are compared
with `/` for `\\`, since Pythrust's `os.path.join` is POSIX on every
platform (docs/grammar.md#deviations-from-python), and groups as sets,
since fclone_lite.py orders a size's groups by SHA-256 and fclone_fast
by xxh3. Exits 1 on any difference.
"""

import subprocess
import sys
import tempfile
from pathlib import Path

Group = tuple[str, frozenset[str]]

TREE = {
    "one": "ab",
    "a/two": "ab",
    "a/b/three": "cd",
    "four": "cd",
    "five": "ef",
    "empty": "",
    "a/empty": "",
}
EXPECTED_GROUPS = 3


def groups(out: str) -> frozenset[Group]:
    """The groups printed in `out`: each `N bytes:` line with its set of
    paths, `\\` read as `/`."""
    lines = out.replace("\\", "/").splitlines()
    blocks = "\n".join(line.rstrip() for line in lines).split("\n\n")
    found = []
    for block in blocks:
        head, *paths = block.strip("\n").split("\n")
        if head:
            found.append((head, frozenset(p.strip() for p in paths)))
    return frozenset(found)


def compare(fast: str, lite: str, expected: int) -> list[str]:
    """What differs between fclone_fast's output `fast` and
    fclone_lite.py's `lite`, which must hold `expected` groups."""
    want = groups(lite)
    if len(want) != expected:
        return [f"fclone_lite.py found {len(want)} groups, expected {expected} groups:\n{lite}"]
    if groups(fast) != want:
        return [f"fclone_fast found other groups than fclone_lite.py:\n{fast}\n---\n{lite}"]
    return []


def run(command: list[str], cwd: Path) -> str:
    """The stdout of `command` run in `cwd`; raises if it fails."""
    done = subprocess.run(command, cwd=cwd, capture_output=True, text=True, check=False)
    if done.returncode != 0:
        raise RuntimeError(f"{command} exited {done.returncode}:\n{done.stderr}")
    return done.stdout


def main(fast: str, oracle: str) -> int:
    """Builds the tree, runs both programs over it, and prints each
    difference; 1 if there are any."""
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        for name, text in TREE.items():
            path = root / "tree" / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(text.encode())
        lite = run([sys.executable, "-I", "-X", "utf8", str(Path(oracle).resolve()), "tree"], root)
        got = run([str(Path(fast).resolve()), "tree"], root)
    found = compare(got, lite, EXPECTED_GROUPS)
    for problem in found:
        print(problem)
    if not found:
        print(f"fclone_fast matches fclone_lite.py: {EXPECTED_GROUPS} groups")
    return 1 if found else 0


if __name__ == "__main__":
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    sys.exit(main(sys.argv[1], sys.argv[2]))
