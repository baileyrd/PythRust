# fclone-lite with its hashing on tasks (M6c driver): every candidate
# (a file sharing its size with another) is hashed through one executor's
# `map`, so the pool starts once, not once per size group. Its output is
# fclone_lite.py's. Pythrust runs it unchanged, as CPython does.
import concurrent.futures
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
    # A task's exception would end `map`'s results, so an unreadable
    # file's digest is "", which no group takes.
    try:
        f = open(path, "rb")
        data = f.read()
        f.close()
    except OSError:
        return ""
    return hashlib.sha256(data).hexdigest()


def groups(paths: list[str], digests: list[str], start: int) -> list[list[str]]:
    by_hash: dict[str, list[str]] = {}
    for i in range(len(paths)):
        h = digests[start + i]
        if h == "":
            continue
        if h in by_hash:
            by_hash[h].append(paths[i])
        else:
            by_hash[h] = [paths[i]]
    found: list[list[str]] = []
    for h in sorted(by_hash):
        if len(by_hash[h]) > 1:
            found.append(by_hash[h])
    return found


def main() -> None:
    root = sys.argv[1] if len(sys.argv) > 1 else "."
    by_size: dict[int, list[str]] = {}
    collect(root, by_size)
    sizes: list[int] = []
    candidates: list[str] = []
    for size in sorted(by_size):
        if len(by_size[size]) < 2:
            continue
        sizes.append(size)
        for path in by_size[size]:
            candidates.append(path)
    with concurrent.futures.ThreadPoolExecutor() as ex:
        digests = list(ex.map(digest, candidates))
    start = 0
    first = True
    for size in sizes:
        for group in groups(by_size[size], digests, start):
            if not first:
                print()
            first = False
            print(size, "bytes:")
            for path in group:
                print(" ", path)
        start += len(by_size[size])


main()
