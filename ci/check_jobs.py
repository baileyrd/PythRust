"""Checks that CI's workflow parses and declares every job it should.

Usage: python3 ci/check_jobs.py [.github/workflows/ci.yml]

A workflow GitHub cannot parse runs no jobs, and no jobs is not green
(Nano, 2026-11-02, after #88's CI never ran). This runs from its own
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

EXPECTED_JOBS = {"check", "memcheck", "miri", "race", "rust-version", "all"}
EXPECTED_OS = {"ubuntu-latest", "ubuntu-24.04-arm", "macos-latest", "windows-latest"}
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
    os = jobs.get("check", {}).get("strategy", {}).get("matrix", {}).get("os", [])
    if set(os) != EXPECTED_OS:
        found.append(f"`check` runs on {sorted(os)}, expected {sorted(EXPECTED_OS)}")
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
    push = on.get("push") or {}
    if "main" not in push.get("branches", []):
        found.append("CI does not run on pushes to `main`")
    return found


def main(path: str) -> int:
    """Prints each problem with ci.yml at `path`; 1 if there are any."""
    try:
        workflow = yaml.load(Path(path).read_text(), Loader=StrictLoader)
    except (OSError, yaml.YAMLError) as e:
        print(f"{path}: cannot read: {e}")
        return 1
    found = problems(workflow)
    for p in found:
        print(f"{path}: {p}")
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else ".github/workflows/ci.yml"))
