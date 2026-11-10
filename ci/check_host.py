"""Fail CI if its native tools are emulated or target a different host.

Usage: python ci/check_host.py RUST_HOST {x64,arm64}
The matrix declares the intended host; inspect the runner, CPython process,
and both Rust tools before the tests build and run native corpus programs.
"""

import os
import platform
import struct
import subprocess
import sys
import sysconfig


MACHINES = {"x86_64": "x64", "amd64": "x64", "aarch64": "arm64", "arm64": "arm64"}
HOSTS = {
    ("Linux", "x64"): "x86_64-unknown-linux-gnu",
    ("Linux", "arm64"): "aarch64-unknown-linux-gnu",
    ("Darwin", "x64"): "x86_64-apple-darwin",
    ("Darwin", "arm64"): "aarch64-apple-darwin",
    ("Windows", "x64"): "x86_64-pc-windows-msvc",
    ("Windows", "arm64"): "aarch64-pc-windows-msvc",
}


def problems(target: str, arch: str, runner: str, system: str, machine: str,
             bits: int, implementation: str, version: tuple[int, int],
             rustc: str, cargo: str) -> list[str]:
    """Compare observed host identities, without executing any tools."""
    found = []
    if runner.lower() != arch:
        found.append(f"runner architecture {runner!r}, expected {arch}")
    if HOSTS.get((system, arch)) != target:
        found.append(f"runner OS {system!r} and architecture {arch!r} do not match {target}")
    if MACHINES.get(machine.lower()) != arch or bits != 64:
        found.append(f"Python process is {machine}/{bits}-bit, expected native {arch}/64-bit")
    if implementation != "CPython" or version != (3, 13):
        found.append(f"oracle is {implementation} {version}, expected CPython 3.13")
    for tool, output in [("rustc", rustc), ("cargo", cargo)]:
        hosts = [line.removeprefix("host: ") for line in output.splitlines() if line.startswith("host: ")]
        if hosts != [target]:
            found.append(f"{tool} hosts are {hosts}, expected {target}")
    return found


def process_machine(system: str) -> str:
    """Windows platform.machine() describes the hardware, even under x64
    emulation on ARM64. sysconfig names the running CPython build instead.
    Unix machine() describes this process (including Rosetta on macOS).
    """
    if system == "Windows":
        return {"win-amd64": "AMD64", "win-arm64": "ARM64"}.get(
            sysconfig.get_platform().lower(), "unknown Windows Python build"
        )
    return platform.machine()


def main(target: str, arch: str) -> int:
    try:
        outputs = [subprocess.check_output([tool, "-vV"], text=True) for tool in ("rustc", "cargo")]
    except (OSError, subprocess.CalledProcessError) as e:
        print(f"cannot inspect Rust host: {e}")
        return 1
    runner = os.environ.get("RUNNER_ARCH", "")
    system = platform.system()
    machine = process_machine(system)
    bits = struct.calcsize("P") * 8
    implementation, version = platform.python_implementation(), sys.version_info[:2]
    print(f"runner: {system} {runner}; Python: {implementation} {version} {machine}/{bits}-bit", flush=True)
    for output in outputs:
        print(output, flush=True)
    found = problems(target, arch, runner, system, machine, bits, implementation, version, *outputs)
    for problem in found:
        print(problem)
    return 1 if found else 0


if __name__ == "__main__":
    if len(sys.argv) != 3:
        sys.exit("usage: python ci/check_host.py RUST_HOST {x64,arm64}")
    sys.exit(main(*sys.argv[1:]))
