"""Release failures must stop before publication or GitHub Release creation."""

import unittest
from unittest.mock import patch

import check_package
import release


class ReleasePolicyTests(unittest.TestCase):
    def test_only_canonical_existing_tag_syntax_is_accepted(self):
        self.assertEqual(release.tag_version("v0.1.0"), "0.1.0")
        for value in ("main", "v01.0.0", "v1.0", "1.0.0", "v1.0.0-rc.1", "v1.0.0\n"):
            with self.subTest(value=value), self.assertRaises(ValueError):
                release.tag_version(value)

    def test_package_excludes_comparison(self):
        base = ["Cargo.toml", "Cargo.lock", "src/lib.rs", "README.md"]
        check_package.check_archive_names(base)
        for forbidden in ("comparison/Cargo.toml", "scripts/release.py", "build.rs"):
            with self.subTest(forbidden=forbidden), self.assertRaises(ValueError):
                check_package.check_archive_names(base + [forbidden])

    def test_registry_checksum_must_match(self):
        with patch.object(release, "registry_version", return_value={"cksum": "other"}):
            with self.assertRaises(ValueError):
                release.assert_published("0.1.0", "expected", 0)

    def test_moved_tag_is_rejected(self):
        with patch.object(release, "git", side_effect=["ref", "changed"]):
            with self.assertRaises(ValueError):
                release.resolve("v0.1.0", "checked")

    def test_manifest_version_must_match_tag(self):
        manifest = '[package]\nname="z58"\nversion="0.2.0"\n'
        with patch.object(release, "git", side_effect=["ref", "checked", manifest]):
            with self.assertRaises(ValueError):
                release.resolve("v0.1.0", "checked")

    def test_registry_must_contain_version(self):
        with patch.object(release, "registry_version", return_value=None):
            with self.assertRaises(ValueError):
                release.assert_published("0.1.0", "expected", 0)

    def test_registry_yank_is_not_success(self):
        with patch.object(release, "registry_version", return_value={"cksum": "same", "yanked": True}):
            with self.assertRaises(ValueError):
                release.assert_published("0.1.0", "same", 0)

    def test_only_confirmed_checksum_succeeds(self):
        with patch.object(release, "registry_version", return_value={"cksum": "same", "yanked": False}):
            release.assert_published("0.1.0", "same", 0)


if __name__ == "__main__":
    unittest.main()
