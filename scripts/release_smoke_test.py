#!/usr/bin/env python3
"""Focused guards for the packaged-release smoke runner."""

import hashlib
from pathlib import Path
import tempfile
import unittest
import zipfile

import release_smoke


class ReleaseSmokeTests(unittest.TestCase):
    def test_checksum_requires_the_exact_archive_name_and_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            archive = root / "blit-test.zip"
            archive.write_bytes(b"archive bytes")
            digest = hashlib.sha256(archive.read_bytes()).hexdigest()
            checksum = root / "blit-test.zip.sha256"
            checksum.write_text(f"{digest}  {archive.name}\n", encoding="utf-8")
            self.assertEqual(release_smoke.verify_checksum(archive, checksum), digest)

            checksum.write_text(f"{digest}  wrong.zip\n", encoding="utf-8")
            with self.assertRaisesRegex(release_smoke.SmokeError, "checksum names"):
                release_smoke.verify_checksum(archive, checksum)

    def test_zip_extraction_rejects_parent_traversal(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            archive = root / "blit-test.zip"
            with zipfile.ZipFile(archive, "w") as bundle:
                bundle.writestr("../escaped.txt", b"no")
            with self.assertRaisesRegex(release_smoke.SmokeError, "unsafe archive"):
                release_smoke.extract_archive(archive, root / "extract")
            self.assertFalse((root / "escaped.txt").exists())

    def test_zip_extraction_rejects_symlinks(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            archive = root / "blit-test.zip"
            member = zipfile.ZipInfo("blit-test/blit")
            member.create_system = 3
            member.external_attr = 0o120777 << 16
            with zipfile.ZipFile(archive, "w") as bundle:
                bundle.writestr(member, "target")
            with self.assertRaisesRegex(release_smoke.SmokeError, "symlink"):
                release_smoke.extract_archive(archive, root / "extract")


if __name__ == "__main__":
    unittest.main()
