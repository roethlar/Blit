#!/usr/bin/env python3
"""Snapshot and refusal guards for package-manager stub generation."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import generate

HERE = Path(__file__).resolve().parent
GOLDEN = HERE / "testdata" / "v0.0.0"
SIDECARS = HERE / "testdata" / "sidecars"

VERSION = "0.0.0"
TAG_SHA = "abcdef012345"
SOURCE_SHA = "d" * 64

LINUX = "a" * 64
MAC = "b" * 64
WINDOWS = "c" * 64


def write_sidecars(directory: Path, *, linux: str, mac: str, windows: str) -> None:
    directory.mkdir(parents=True, exist_ok=True)
    (directory / f"{generate.LINUX_ARCHIVE}.sha256").write_text(
        f"{linux}  {generate.LINUX_ARCHIVE}\n", encoding="utf-8"
    )
    (directory / f"{generate.MAC_ARCHIVE}.sha256").write_text(
        f"{mac}  {generate.MAC_ARCHIVE}\n", encoding="utf-8"
    )
    (directory / f"{generate.WINDOWS_ARCHIVE}.sha256").write_text(
        f"{windows}  {generate.WINDOWS_ARCHIVE}\n", encoding="utf-8"
    )


class GenerateTests(unittest.TestCase):
    def test_snapshots_match_golden_tree(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            out = Path(temp) / "out"
            generate.generate(
                version=VERSION,
                tag_sha=TAG_SHA,
                digests=generate.load_sidecars(SIDECARS),
                source_sha256=SOURCE_SHA,
                out=out,
            )
            golden_files = sorted(p for p in GOLDEN.rglob("*") if p.is_file())
            self.assertTrue(golden_files, "golden tree is empty")
            for golden in golden_files:
                relative = golden.relative_to(GOLDEN)
                actual = out / relative
                self.assertTrue(actual.is_file(), f"missing generated file: {relative}")
                self.assertEqual(
                    actual.read_text(encoding="utf-8"),
                    golden.read_text(encoding="utf-8"),
                    f"generated {relative} drifted from testdata",
                )

    def test_changing_a_digest_changes_the_stub_that_embeds_it(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            sidecars = root / "sidecars"
            write_sidecars(sidecars, linux=LINUX, mac=MAC, windows=WINDOWS)
            out = root / "out"
            generate.generate(
                version=VERSION,
                tag_sha=TAG_SHA,
                digests=generate.load_sidecars(sidecars),
                source_sha256=SOURCE_SHA,
                out=out,
            )
            original = (out / "aur" / "blit-bin" / "PKGBUILD").read_text(encoding="utf-8")
            self.assertIn(LINUX, original)

            write_sidecars(sidecars, linux="e" * 64, mac=MAC, windows=WINDOWS)
            generate.generate(
                version=VERSION,
                tag_sha=TAG_SHA,
                digests=generate.load_sidecars(sidecars),
                source_sha256=SOURCE_SHA,
                out=out,
            )
            rewritten = (out / "aur" / "blit-bin" / "PKGBUILD").read_text(encoding="utf-8")
            self.assertNotEqual(rewritten, original)
            self.assertIn("e" * 64, rewritten)
            self.assertNotIn(LINUX, rewritten)

    def test_missing_platform_digest_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            sidecars = Path(temp) / "sidecars"
            write_sidecars(sidecars, linux=LINUX, mac=MAC, windows=WINDOWS)
            (sidecars / f"{generate.MAC_ARCHIVE}.sha256").unlink()
            with self.assertRaisesRegex(generate.GenerateError, "missing digest"):
                generate.generate(
                    version=VERSION,
                    tag_sha=TAG_SHA,
                    digests=generate.load_sidecars(sidecars),
                    source_sha256=SOURCE_SHA,
                    out=Path(temp) / "out",
                )

    def test_source_stubs_export_the_tag_sha(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            out = Path(temp) / "out"
            generate.generate(
                version=VERSION,
                tag_sha=TAG_SHA,
                digests=generate.load_sidecars(SIDECARS),
                source_sha256=SOURCE_SHA,
                out=out,
            )
            core = (out / "homebrew" / "blit.rb").read_text(encoding="utf-8")
            aur = (out / "aur" / "blit" / "PKGBUILD").read_text(encoding="utf-8")
            self.assertIn(f'ENV["BLIT_GIT_SHA"] = "{TAG_SHA}"', core)
            self.assertIn(f"export BLIT_GIT_SHA={TAG_SHA}", aur)
            bin_formula = (out / "homebrew" / "blit-bin.rb").read_text(encoding="utf-8")
            self.assertNotIn("cargo build", bin_formula)

    def test_garbage_version_and_sha_are_refused(self) -> None:
        with self.assertRaisesRegex(generate.GenerateError, "N.N.N"):
            generate.validate_version("v0.0.0")
        with self.assertRaisesRegex(generate.GenerateError, "12 lowercase"):
            generate.validate_tag_sha("ABCDEF012345")


if __name__ == "__main__":
    unittest.main()
