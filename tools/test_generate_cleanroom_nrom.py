"""Adversarial regression tests for the clean-room NROM generator."""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent
GENERATOR = PROJECT_ROOT / "tools" / "generate-cleanroom-nrom.py"
GENERATED = PROJECT_ROOT / "crates" / "retro-testkit" / "src" / "cleanroom_nrom.rs"
MAX_PINNED_FILE_BYTES = 1_000_000
REQUIRED_PY65_FILES = (
    "LICENSE.txt",
    "py65/__init__.py",
    "py65/devices/__init__.py",
    "py65/devices/mpu6502.py",
    "py65/utils/__init__.py",
    "py65/utils/conversions.py",
    "py65/utils/devices.py",
)


def run_generator(py65_root: Path, output: Path, *, check: bool = False):
    command = [
        sys.executable,
        str(GENERATOR),
        "--py65-root",
        str(py65_root),
        "--output",
        str(output),
    ]
    if check:
        command.append("--check")
    return subprocess.run(
        command, capture_output=True, text=True, check=False, timeout=60
    )


class GeneratorTests(unittest.TestCase):
    py65_root: Path

    def test_generation_is_byte_identical(self) -> None:
        with tempfile.TemporaryDirectory(prefix="panda-nrom-generate-") as directory:
            first = Path(directory) / "first.rs"
            second = Path(directory) / "second.rs"
            first_result = run_generator(self.py65_root, first)
            second_result = run_generator(self.py65_root, second)
            self.assertEqual(first_result.returncode, 0, first_result.stderr)
            self.assertEqual(second_result.returncode, 0, second_result.stderr)
            self.assertEqual(first.read_bytes(), second.read_bytes())
            self.assertEqual(first.read_bytes(), GENERATED.read_bytes())

    def test_check_accepts_current_output_without_modifying_it(self) -> None:
        before = GENERATED.read_bytes()
        result = run_generator(self.py65_root, GENERATED, check=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("verified 3 clean-room cases", result.stdout)
        self.assertEqual(GENERATED.read_bytes(), before)

    def test_check_rejects_stale_and_missing_output_without_writing(self) -> None:
        with tempfile.TemporaryDirectory(prefix="panda-nrom-check-") as directory:
            stale = Path(directory) / "stale.rs"
            stale_bytes = GENERATED.read_bytes() + b"// stale\n"
            stale.write_bytes(stale_bytes)
            result = run_generator(self.py65_root, stale, check=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("output is stale", result.stderr)
            self.assertEqual(stale.read_bytes(), stale_bytes)

            truncated = Path(directory) / "truncated.rs"
            truncated_bytes = GENERATED.read_bytes()[:-1]
            truncated.write_bytes(truncated_bytes)
            result = run_generator(self.py65_root, truncated, check=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("output is stale", result.stderr)
            self.assertEqual(truncated.read_bytes(), truncated_bytes)

            missing = Path(directory) / "missing.rs"
            result = run_generator(self.py65_root, missing, check=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("missing or unreadable", result.stderr)
            self.assertFalse(missing.exists())

    def test_oversized_source_is_rejected_before_output(self) -> None:
        with tempfile.TemporaryDirectory(prefix="panda-nrom-oversized-") as directory:
            root = Path(directory) / "py65"
            root.mkdir()
            output = Path(directory) / "generated.rs"

            (root / "LICENSE.txt").write_bytes(b"x" * MAX_PINNED_FILE_BYTES)
            result = run_generator(root, output)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("pinned py65 hash mismatch", result.stderr)
            self.assertNotIn("file is oversized", result.stderr)
            self.assertFalse(output.exists())

            (root / "LICENSE.txt").write_bytes(b"x" * (MAX_PINNED_FILE_BYTES + 1))
            result = run_generator(root, output)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("pinned py65 file is oversized", result.stderr)
            self.assertFalse(output.exists())

    def test_missing_source_is_rejected_before_output(self) -> None:
        with tempfile.TemporaryDirectory(prefix="panda-nrom-missing-source-") as directory:
            root = Path(directory) / "py65"
            root.mkdir()
            output = Path(directory) / "generated.rs"
            result = run_generator(root, output)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("pinned py65 file is missing or unreadable", result.stderr)
            self.assertFalse(output.exists())

    def test_same_length_source_mutation_is_rejected_before_output(self) -> None:
        with tempfile.TemporaryDirectory(prefix="panda-nrom-mutated-") as directory:
            root = Path(directory) / "py65"
            for relative in REQUIRED_PY65_FILES:
                source = self.py65_root / relative
                destination = root / relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(source, destination)

            cpu = root / "py65/devices/mpu6502.py"
            mutated = bytearray(cpu.read_bytes())
            mutated[0] ^= 1
            cpu.write_bytes(mutated)
            output = Path(directory) / "generated.rs"
            result = run_generator(root, output)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("pinned py65 hash mismatch", result.stderr)
            self.assertFalse(output.exists())


def main() -> None:
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument("--py65-root", required=True, type=Path)
    args, unittest_args = parser.parse_known_args()
    GeneratorTests.py65_root = args.py65_root.resolve(strict=True)
    unittest.main(argv=[sys.argv[0], *unittest_args])


if __name__ == "__main__":
    main()
