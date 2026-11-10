"""Tests for check_jobs: python3 -m unittest discover -s ci"""

import copy
import tempfile
import unittest
from pathlib import Path

import check_jobs

ROOT = Path(__file__).resolve().parent.parent
CI = ROOT / ".github" / "workflows" / "ci.yml"
GUARD = ROOT / ".github" / "workflows" / "ci-jobs.yml"


def sound() -> dict:
    jobs = {j: {} for j in check_jobs.EXPECTED_JOBS}
    jobs["check"] = {"strategy": {"matrix": {"include": [
        {"os": os, "target": target, "architecture": arch}
        for os, (target, arch) in check_jobs.EXPECTED_TARGETS.items()
    ]}}}
    jobs["all"] = {"needs": sorted(check_jobs.EXPECTED_JOBS - {"all"})}
    for body in jobs.values():
        body["runs-on"] = "ubuntu-latest"
    return {True: {"push": {"branches": ["main"]}, "pull_request": None}, "jobs": jobs}


class Problems(unittest.TestCase):
    def test_the_repository_workflow_is_sound(self) -> None:
        self.assertEqual(check_jobs.main(str(CI), str(GUARD)), 0)

    def test_a_sound_workflow_has_none(self) -> None:
        self.assertEqual(check_jobs.problems(sound()), [])

    def test_a_missing_job_is_one(self) -> None:
        w = sound()
        del w["jobs"]["race"]
        found = check_jobs.problems(w)
        self.assertTrue(any("jobs are" in p for p in found), found)

    def test_a_missing_platform_is_one(self) -> None:
        w = sound()
        w["jobs"]["check"]["strategy"]["matrix"]["include"].pop()
        self.assertEqual(len(check_jobs.problems(w)), 1)

    def test_a_wrong_or_duplicate_native_lane_is_one(self) -> None:
        for field, value in [("target", "x86_64-pc-windows-msvc"), ("architecture", "x64")]:
            w = sound()
            w["jobs"]["check"]["strategy"]["matrix"]["include"][-1][field] = value
            self.assertEqual(len(check_jobs.problems(w)), 1)
        w = sound()
        lanes = w["jobs"]["check"]["strategy"]["matrix"]["include"]
        lanes.append(lanes[-1])
        self.assertEqual(len(check_jobs.problems(w)), 1)

    def test_excluding_a_native_lane_is_one(self) -> None:
        w = sound()
        w["jobs"]["check"]["strategy"]["matrix"]["exclude"] = [{"os": "windows-11-arm"}]
        self.assertEqual(len(check_jobs.problems(w)), 1)

    def test_a_gate_missing_a_job_is_one(self) -> None:
        w = copy.deepcopy(sound())
        w["jobs"]["all"]["needs"].remove("miri")
        self.assertEqual(len(check_jobs.problems(w)), 1)

    def test_no_jobs_is_one(self) -> None:
        self.assertEqual(check_jobs.problems({True: "push"}), ["the workflow has no `jobs` mapping"])
        self.assertEqual(len(check_jobs.problems(None)), 1)

    def test_a_missing_trigger_is_one(self) -> None:
        w = sound()
        del w[True]["pull_request"]
        self.assertEqual(check_jobs.problems(w), ["CI does not run on `pull_request`"])

    def test_a_path_filter_is_one(self) -> None:
        w = sound()
        w[True]["pull_request"] = {"paths": ["crates/**"]}
        found = check_jobs.problems(w)
        self.assertEqual(len(found), 1)
        self.assertIn("every PR", found[0])

    def test_a_filtered_guard_fails(self) -> None:
        with tempfile.TemporaryDirectory() as d:
            guard = Path(d) / "ci-jobs.yml"
            guard.write_text(GUARD.read_text().replace("  pull_request:\n", "  pull_request:\n    paths: [ci/**]\n"))
            self.assertEqual(check_jobs.main(str(CI), str(guard)), 1)

    def test_a_job_without_a_runner_is_one(self) -> None:
        w = sound()
        w["jobs"]["miri"] = {"runs_on": "ubuntu-latest"}
        self.assertEqual(check_jobs.problems(w), ["job `miri` has no `runs-on`"])

    def test_a_duplicate_job_fails(self) -> None:
        with tempfile.TemporaryDirectory() as d:
            dup = Path(d) / "ci.yml"
            dup.write_text(CI.read_text() + "\n  race:\n    runs-on: ubuntu-latest\n")
            self.assertEqual(check_jobs.main(str(dup)), 1)

    def test_unparsable_yaml_fails(self) -> None:
        with tempfile.TemporaryDirectory() as d:
            bad = Path(d) / "ci.yml"
            bad.write_text("jobs:\n  race:\n    - run: a -- b:: c\n   bad: [\n")
            self.assertEqual(check_jobs.main(str(bad)), 1)


if __name__ == "__main__":
    unittest.main()
