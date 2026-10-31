# fclone-lite: print groups of identical files under a directory.
# The M6 driver (docs/roadmap.md#phase-5-m6-plan). This is the CPython
# version, the oracle for the Pythrust one; it keeps to what Pythrust
# plans to support, so the two can converge on one source.
import hashlib
import os
import sys


def collect(root: str, by_size: dict[int, list[str]]) -> None:
    try:
        names = sorted(os.listdir(root))
    except OSError:
        return
    for name in names:
        path = os.path.join(root, name)
        if os.path.islink(path):
            continue
        if os.path.isdir(path):
            collect(path, by_size)
        elif os.path.isfile(path):
            size = os.path.getsize(path)
            if size in by_size:
                by_size[size].append(path)
            else:
                by_size[size] = [path]


def digest(path: str) -> str:
    f = open(path, "rb")
    data = f.read()
    f.close()
    return hashlib.sha256(data).hexdigest()


def groups(paths: list[str]) -> list[list[str]]:
    by_hash: dict[str, list[str]] = {}
    for path in paths:
        try:
            h = digest(path)
        except OSError:
            continue
        if h in by_hash:
            by_hash[h].append(path)
        else:
            by_hash[h] = [path]
    found: list[list[str]] = []
    for h in sorted(by_hash):
        if len(by_hash[h]) > 1:
            found.append(by_hash[h])
    return found


def main() -> None:
    root = sys.argv[1] if len(sys.argv) > 1 else "."
    by_size: dict[int, list[str]] = {}
    collect(root, by_size)
    first = True
    for size in sorted(by_size):
        if len(by_size[size]) < 2:
            continue
        for group in groups(by_size[size]):
            if not first:
                print()
            first = False
            print(size, "bytes:")
            for path in group:
                print(" ", path)


main()
