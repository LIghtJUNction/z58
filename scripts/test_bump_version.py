"""Release version-selection tests; no registry access or repository writes."""

import unittest

from bump_version import rewrite, rewrite_comparison_lock


MANIFEST = '''[package]
name = "z58"
version = "0.1.0"
edition = "2021"
'''
LOCK = '''version = 3

[[package]]
name = "z58"
version = "0.1.0"
'''

COMPARISON_LOCK = '''version = 4

[[package]]
name = "z58"
version = "0.1.0"

[[package]]
name = "z58-comparison"
version = "0.0.0"
dependencies = ["z58"]
'''


class BumpVersionTests(unittest.TestCase):
    def test_current_is_unchanged_for_first_release(self):
        self.assertEqual(rewrite(MANIFEST, LOCK, "current"), ("0.1.0", MANIFEST, LOCK))

    def test_semantic_bumps_update_both_files(self):
        for mode, expected in (("patch", "0.1.1"), ("minor", "0.2.0"), ("major", "1.0.0")):
            with self.subTest(mode=mode):
                version, manifest, lock = rewrite(MANIFEST, LOCK, mode)
                self.assertEqual(version, expected)
                self.assertIn(f'version = "{expected}"', manifest)
                self.assertIn(f'version = "{expected}"', lock)

    def test_nonzero_patch_resets_on_minor(self):
        manifest = MANIFEST.replace("0.1.0", "2.7.9")
        lock = LOCK.replace("0.1.0", "2.7.9")
        self.assertEqual(rewrite(manifest, lock, "minor")[0], "2.8.0")
        self.assertEqual(rewrite(manifest, lock, "major")[0], "3.0.0")

    def test_comparison_lock_updates_only_the_local_crate(self):
        updated = rewrite_comparison_lock(COMPARISON_LOCK, "0.1.2")
        self.assertIn('name = "z58"\nversion = "0.1.1"', updated)
        self.assertIn('name = "z58-comparison"\nversion = "0.0.0"', updated)

    def test_rejects_mismatched_lock(self):
        with self.assertRaisesRegex(ValueError, "Cargo.lock"):
            rewrite(MANIFEST, LOCK.replace("0.1.0", "0.2.0"), "patch")

    def test_rejects_noncanonical_version(self):
        with self.assertRaisesRegex(ValueError, "canonical"):
            rewrite(MANIFEST.replace("0.1.0", "0.01.0"), LOCK, "patch")

    def test_rejects_duplicate_version_entry(self):
        with self.assertRaisesRegex(ValueError, "exactly one"):
            rewrite(MANIFEST + '\n[package.metadata.notes]\ntext = \'version = "0.1.0"\'\n', LOCK, "patch")

    def test_rejects_unknown_modes_and_wrong_package(self):
        with self.assertRaises(ValueError):
            rewrite(MANIFEST, LOCK, "beta")
        with self.assertRaisesRegex(ValueError, "z58"):
            rewrite(MANIFEST.replace('name = "z58"', 'name = "wrong"'), LOCK, "patch")
