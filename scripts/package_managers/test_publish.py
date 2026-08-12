#!/usr/bin/env python3
"""Guards for package-manager publish skip/fail isolation."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import generate
import publish


class FakePublisher:
    def __init__(self) -> None:
        self.calls: list[str] = []

    def homebrew_tap(self, stub_dir: Path, token: str) -> str:
        self.calls.append(f"homebrew-tap:{token}:{stub_dir.name}")
        return "ok-tap"

    def scoop(self, stub_dir: Path, token: str) -> str:
        self.calls.append(f"scoop:{token}")
        return "ok-scoop"

    def aur(self, stub_dir: Path, token: str) -> str:
        self.calls.append(f"aur:{token}")
        return "ok-aur"

    def winget(self, stub_dir: Path, token: str) -> str:
        self.calls.append(f"winget:{token}")
        return "ok-winget"

    def homebrew_core(self, stub_dir: Path, token: str) -> str:
        self.calls.append(f"homebrew-core:{token}")
        raise publish.PublishError("fork missing")


class PublishTests(unittest.TestCase):
    def test_empty_secrets_skip_every_channel(self) -> None:
        actor = FakePublisher()
        outcomes = publish.run_channels(Path("unused"), {}, publisher=actor)
        self.assertEqual([item.status for item in outcomes], ["skipped"] * 5)
        self.assertEqual(actor.calls, [])
        self.assertTrue(all("is empty" in item.detail for item in outcomes))

    def test_only_configured_channels_run(self) -> None:
        actor = FakePublisher()
        outcomes = publish.run_channels(
            Path("stubs"),
            {"HOMEBREW_TAP_TOKEN": "tap-token", "AUR_SSH_KEY": "  "},
            publisher=actor,
        )
        by_name = {item.name: item for item in outcomes}
        self.assertEqual(by_name["homebrew-tap"].status, "updated")
        self.assertEqual(by_name["aur"].status, "skipped")
        self.assertEqual(actor.calls, ["homebrew-tap:tap-token:stubs"])

    def test_failed_channel_does_not_stop_the_others(self) -> None:
        actor = FakePublisher()
        outcomes = publish.run_channels(
            Path("stubs"),
            {
                "HOMEBREW_TAP_TOKEN": "tap-token",
                "HOMEBREW_CORE_TOKEN": "core-token",
                "SCOOP_BUCKET_TOKEN": "scoop-token",
            },
            publisher=actor,
        )
        by_name = {item.name: item for item in outcomes}
        self.assertEqual(by_name["homebrew-tap"].status, "updated")
        self.assertEqual(by_name["scoop"].status, "updated")
        self.assertEqual(by_name["homebrew-core"].status, "failed")
        self.assertIn("fork missing", by_name["homebrew-core"].detail)
        self.assertEqual(
            actor.calls,
            ["homebrew-tap:tap-token:stubs", "scoop:scoop-token", "homebrew-core:core-token"],
        )

    def test_whitespace_secret_is_treated_as_missing(self) -> None:
        self.assertIsNone(publish.read_secret("X", {"X": "  \n"}))
        self.assertEqual(publish.read_secret("X", {"X": "token"}), "token")
        self.assertEqual(publish.env_or("REPO", "default", {"REPO": ""}), "default")
        self.assertEqual(publish.env_or("REPO", "default", {"REPO": "owner/tap"}), "owner/tap")

    def test_normalize_tag_strips_v(self) -> None:
        self.assertEqual(publish.normalize_tag("v0.1.2"), ("v0.1.2", "0.1.2"))
        self.assertEqual(publish.normalize_tag("0.1.2"), ("v0.1.2", "0.1.2"))
        with self.assertRaises(publish.PublishError):
            publish.normalize_tag("latest")

    def test_prepare_stubs_from_local_sidecars_does_not_need_network(self) -> None:
        here = Path(__file__).resolve().parent
        with tempfile.TemporaryDirectory() as temp:
            source = Path(temp) / "src.tar.gz"
            source.write_bytes(b"source-bytes")
            stub_dir = publish.prepare_stubs(
                tag="v0.0.0",
                out=Path(temp) / "out",
                repo_root=here,
                sidecars=here / "testdata" / "sidecars",
                source_tarball=source,
                tag_sha="abcdef012345",
                release_repo="example/unused",
            )
            self.assertTrue((stub_dir / "homebrew" / "blit-bin.rb").is_file())
            pkgbuild = (stub_dir / "aur" / "blit" / "PKGBUILD").read_text(encoding="utf-8")
            self.assertIn("export BLIT_GIT_SHA=abcdef012345", pkgbuild)
            self.assertEqual(
                generate.sha256_file(source),
                generate.sha256_file(source),
            )


if __name__ == "__main__":
    unittest.main()
