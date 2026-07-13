"""Acquire bounded pinned oracle sources and verify clean-room generated data."""

from __future__ import annotations

import subprocess
import sys
import tempfile
import urllib.error
import urllib.request
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent
PY65_COMMIT = "3138e1b337734a9b2ac1ea90ee7a453514436221"
MAX_DOWNLOAD_BYTES = 1_000_000
DOWNLOAD_TIMEOUT_SECONDS = 30
PY65_FILES = (
    "LICENSE.txt",
    "py65/__init__.py",
    "py65/devices/__init__.py",
    "py65/devices/mpu6502.py",
    "py65/utils/__init__.py",
    "py65/utils/conversions.py",
    "py65/utils/devices.py",
)


def download_pinned_sources(root: Path) -> None:
    base = f"https://raw.githubusercontent.com/mnaberez/py65/{PY65_COMMIT}"
    for relative in PY65_FILES:
        request = urllib.request.Request(
            f"{base}/{relative}",
            headers={"User-Agent": "PandaUniEmu-cleanroom-check/1"},
        )
        try:
            with urllib.request.urlopen(
                request, timeout=DOWNLOAD_TIMEOUT_SECONDS
            ) as response:
                declared = response.headers.get("Content-Length")
                if declared is not None and int(declared) > MAX_DOWNLOAD_BYTES:
                    raise RuntimeError(f"pinned py65 download is oversized: {relative}")
                data = response.read(MAX_DOWNLOAD_BYTES + 1)
        except (OSError, ValueError, urllib.error.URLError) as error:
            raise RuntimeError(f"pinned py65 download failed: {relative}") from error
        if len(data) > MAX_DOWNLOAD_BYTES:
            raise RuntimeError(f"pinned py65 download is oversized: {relative}")
        destination = root / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(data)


def run_checked(command: list[str]) -> None:
    result = subprocess.run(command, cwd=PROJECT_ROOT, check=False, timeout=120)
    if result.returncode != 0:
        raise RuntimeError(f"clean-room verification command failed with {result.returncode}")


def main() -> int:
    try:
        with tempfile.TemporaryDirectory(prefix="panda-py65-") as directory:
            py65_root = Path(directory)
            download_pinned_sources(py65_root)
            run_checked(
                [
                    sys.executable,
                    "tools/test_generate_cleanroom_nrom.py",
                    "--py65-root",
                    str(py65_root),
                ]
            )
            run_checked(
                [
                    sys.executable,
                    "tools/generate-cleanroom-nrom.py",
                    "--py65-root",
                    str(py65_root),
                    "--output",
                    "crates/retro-testkit/src/cleanroom_nrom.rs",
                    "--check",
                ]
            )
    except (OSError, RuntimeError, subprocess.SubprocessError) as error:
        print(f"clean-room verification failed: {error}", file=sys.stderr)
        return 1
    print("clean-room generator verification passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
