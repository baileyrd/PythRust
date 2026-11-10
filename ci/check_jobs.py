"""Checks that CI's workflow parses and declares every job it should.

Usage: python3 ci/check_jobs.py [CI_YML [GUARD_YML]]

A workflow GitHub cannot parse runs no jobs, and no jobs is not green
(Nano, 2026-11-02): #88's first four heads, 3b620f4 to 65320b1, ran no
CI jobs because ci.yml did not parse; 204e17b fixed it. This runs from its own
workflow, so it still reports when ci.yml is broken. Needs PyYAML.
"""

import sys
from pathlib import Path

import yaml

class StrictLoader(yaml.SafeLoader):
    """PyYAML's safe loader, but a duplicate key is an error, as GitHub
    treats it, not a silent overwrite."""


def _mapping(loader: StrictLoader, node: yaml.MappingNode) -> dict:
    keys = [loader.construct_object(k) for k, _ in node.value]
    for k in keys:
        if keys.count(k) > 1:
            raise yaml.constructor.ConstructorError(
                None, None, f"duplicate key {k!r}", node.start_mark
            )
    return loader.construct_mapping(node)


StrictLoader.add_constructor(yaml.resolver.BaseResolver.DEFAULT_MAPPING_TAG, _mapping)

EXPECTED_JOBS = {
    "check", "memcheck", "miri", "race", "rust-version", "compiler-rust-version", "driver",
    "all",
}
EXPECTED_TARGETS = {
    "ubuntu-latest": ("x86_64-unknown-linux-gnu", "x64"),
    "ubuntu-24.04-arm": ("aarch64-unknown-linux-gnu", "arm64"),
    "macos-latest": ("aarch64-apple-darwin", "arm64"),
    "windows-latest": ("x86_64-pc-windows-msvc", "x64"),
    "macos-15-intel": ("x86_64-apple-darwin", "x64"),
    "windows-11-arm": ("aarch64-pc-windows-msvc", "arm64"),
}
GATE = "all"


def problems(workflow: object) -> list[str]:
    """What is wrong with the parsed `workflow`; empty when it is sound."""
    if not isinstance(workflow, dict) or not isinstance(workflow.get("jobs"), dict):
        return ["the workflow has no `jobs` mapping"]
    jobs: dict = workflow["jobs"]
    found = triggers(workflow)
    found += [f"job `{j}` has no `runs-on`" for j, body in jobs.items() if "runs-on" not in body]
    if set(jobs) != EXPECTED_JOBS:
        found.append(f"jobs are {sorted(jobs)}, expected {sorted(EXPECTED_JOBS)}")
    matrix = jobs.get("check", {}).get("strategy", {}).get("matrix", {})
    expected = [
        {"os": os, "target": target, "architecture": arch}
        for os, (target, arch) in EXPECTED_TARGETS.items()
    ]
    lanes = matrix.get("include", [])
    if (set(matrix) != {"include"} or not isinstance(lanes, list)
            or len(lanes) != len(expected) or any(lane not in lanes for lane in expected)):
        found.append(f"`check` matrix must contain exactly these native lanes: {expected}")
    needs = set(jobs.get(GATE, {}).get("needs", []))
    if needs != EXPECTED_JOBS - {GATE}:
        found.append(f"`{GATE}` needs {sorted(needs)}, expected every other job")
    return found


def triggers(workflow: dict) -> list[str]:
    """Whether CI runs on pull requests and on pushes to main. YAML 1.1
    reads the bare key `on` as true."""
    on = workflow.get("on", workflow.get(True))
    if not isinstance(on, dict):
        return ["`on` is not a mapping of triggers"]
    found = []
    if "pull_request" not in on:
        found.append("CI does not run on `pull_request`")
    pr = on.get("pull_request") or {}
    filters = sorted(set(pr) & {"paths", "paths-ignore", "branches", "branches-ignore"})
    if filters:
        found.append(f"`pull_request` is filtered by {filters}; it must run on every PR")
    push = on.get("push") or {}
    if "main" not in push.get("branches", []):
        found.append("CI does not run on pushes to `main`")
    return found


def load(path: str) -> object:
    """The workflow at `path`; raises on a file that cannot be read."""
    return yaml.load(Path(path).read_text(), Loader=StrictLoader)


def main(path: str, guard: str | None = None) -> int:
    """Prints each problem with ci.yml at `path`, and with the triggers of
    the guard workflow at `guard`, which must also run on every PR; 1 if
    there are any."""
    found = []
    try:
        found += [f"{path}: {p}" for p in problems(load(path))]
        if guard is not None:
            workflow = load(guard)
            on = triggers(workflow) if isinstance(workflow, dict) else ["not a mapping"]
            found += [f"{guard}: {p}" for p in on]
    except (OSError, yaml.YAMLError) as e:
        found.append(f"cannot read: {e}")
    for p in found:
        print(p)
    return 1 if found else 0


if __name__ == "__main__":
    args = sys.argv[1:] or [".github/workflows/ci.yml", ".github/workflows/ci-jobs.yml"]
    sys.exit(main(*args[:2]))
