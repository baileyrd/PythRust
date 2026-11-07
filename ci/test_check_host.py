"""Native host checks must reject emulated tools and the wrong oracle."""

import contextlib
import io
import subprocess
import unittest
from unittest.mock import patch

import check_host


def observed(arch: str = "arm64", system: str = "Windows") -> dict:
    host = check_host.HOSTS[(system, arch)]
    return dict(target=host, arch=arch, runner=arch.upper(), system=system,
                machine="ARM64" if arch == "arm64" else "AMD64", bits=64,
                implementation="CPython", version=(3, 13),
                rustc=f"rustc 1.97.0\nhost: {host}\n", cargo=f"cargo 1.97.0\nhost: {host}\n")


class Host(unittest.TestCase):
    def test_all_six_native_hosts(self) -> None:
        for system, arch in check_host.HOSTS:
            with self.subTest(system=system, arch=arch):
                self.assertEqual(check_host.problems(**observed(arch, system)), [])

    def test_unix_machine_spellings(self) -> None:
        for machine, arch in [("x86_64", "x64"), ("aarch64", "arm64")]:
            self.assertEqual(check_host.problems(**(observed(arch, "Linux") | {"machine": machine})), [])

    def test_wrong_identity_fails(self) -> None:
        for change in [dict(runner="X64"), dict(runner=""), dict(system="Linux"),
                       dict(machine="AMD64"), dict(machine="unknown"), dict(bits=32),
                       dict(implementation="PyPy"), dict(version=(3, 14)),
                       dict(target="x86_64-pc-windows-msvc"), dict(arch="unknown")]:
            with self.subTest(change=change):
                self.assertTrue(check_host.problems(**(observed() | change)))

    def test_emulated_missing_or_duplicate_rust_host_fails(self) -> None:
        for tool in ["rustc", "cargo"]:
            for output in ["host: x86_64-pc-windows-msvc\n", "", "host: aarch64-pc-windows-msvc\n" * 2]:
                with self.subTest(tool=tool, output=output):
                    self.assertTrue(check_host.problems(**(observed() | {tool: output})))

    def test_missing_or_failed_tool_fails(self) -> None:
        for error in [FileNotFoundError("rustc"), subprocess.CalledProcessError(1, "rustc")]:
            with patch("check_host.subprocess.check_output", side_effect=error), contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(check_host.main("aarch64-pc-windows-msvc", "arm64"), 1)


if __name__ == "__main__":
    unittest.main()
