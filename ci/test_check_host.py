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

    def test_arm64_windows_hardware_with_x64_python_fails_acquisition(self) -> None:
        # CPython 3.13's platform.machine() can report ARM64 for an x64
        # Python process. Exercise main's acquisition, not just problems().
        host = "aarch64-pc-windows-msvc"
        for build, expected in [("win-arm64", 0), ("win-amd64", 1), ("win32", 1), ("unknown", 1)]:
            with self.subTest(build=build), contextlib.ExitStack() as stack:
                stack.enter_context(patch("check_host.platform.system", return_value="Windows"))
                stack.enter_context(patch("check_host.platform.machine", return_value="ARM64"))
                stack.enter_context(patch("check_host.sysconfig.get_platform", return_value=build))
                stack.enter_context(patch("check_host.struct.calcsize", return_value=8))
                stack.enter_context(patch("check_host.platform.python_implementation", return_value="CPython"))
                stack.enter_context(patch("check_host.sys.version_info", (3, 13, 0)))
                stack.enter_context(patch.dict("check_host.os.environ", {"RUNNER_ARCH": "ARM64"}))
                stack.enter_context(patch("check_host.subprocess.check_output", return_value=f"host: {host}\n"))
                stack.enter_context(contextlib.redirect_stdout(io.StringIO()))
                self.assertEqual(check_host.main(host, "arm64"), expected)

    def test_missing_or_failed_tool_fails(self) -> None:
        for error in [FileNotFoundError("rustc"), subprocess.CalledProcessError(1, "rustc")]:
            with patch("check_host.subprocess.check_output", side_effect=error), contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(check_host.main("aarch64-pc-windows-msvc", "arm64"), 1)


if __name__ == "__main__":
    unittest.main()
