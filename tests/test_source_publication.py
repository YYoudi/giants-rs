from pathlib import Path
import hashlib
import json
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from tools.audit_source_tree import scan


class SourcePublicationTests(unittest.TestCase):
    def git(self, root, *args):
        return subprocess.check_output(['git', '-C', str(root), *args], stderr=subprocess.DEVNULL)

    def initialize(self, root):
        self.git(root, 'init', '--quiet')
        self.git(root, 'config', 'user.name', 'Synthetic Test')
        self.git(root, 'config', 'user.email', 'synthetic@example.invalid')

    def commit(self, root):
        self.git(root, 'add', '.')
        self.git(root, 'commit', '--quiet', '-m', 'Synthetic fixture')

    def test_historical_binary_and_removed_secret_are_still_reported_without_values(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.initialize(root)
            (root/'payload.txt').write_bytes(b'MZ\0 synthetic')
            secret = 'ghp_' + 'a'*36
            (root/'credential.txt').write_text(secret, encoding='utf8')
            self.commit(root)
            (root/'payload.txt').rename(root/'payload.exe')
            (root/'credential.txt').unlink()
            self.commit(root)
            (root/'payload.exe').unlink()
            (root/'README.md').write_text('Synthetic only', encoding='utf8')
            self.commit(root)
            report = scan(root)
            self.assertEqual(report['reachable_commits'], 3)
            self.assertIn('payload.exe', [f['path'] for f in report['findings']])
            flags = {flag for f in report['findings'] for flag in f['flags']}
            self.assertTrue({'binary_nul', 'github_token', 'restricted_path_or_link'} <= flags)
            self.assertNotIn(secret, str(report))

    def test_plain_independent_source_has_no_pattern_findings(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.initialize(root)
            (root/'main.rs').write_text('fn main() {}\n', encoding='utf8')
            self.commit(root)
            self.assertEqual(scan(root)['findings'], [])

    def test_reviewed_alias_exception_is_exact_and_keeps_other_checks(self):
        alias = b'FUN_' + b'00401000'
        for variant in ('reviewed', 'changed', 'other_path', 'secret'):
            with self.subTest(variant=variant), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                self.initialize(root)
                data = b'Reviewed minimal alias: ' + alias
                if variant == 'secret':
                    data += b' ghp_' + b'a' * 36
                approved = {'summary.md': {hashlib.sha256(data).hexdigest()}}
                (root/'summary.md').write_bytes(data)
                self.commit(root)
                if variant == 'changed':
                    (root/'summary.md').write_bytes(data + b' appended')
                    self.commit(root)
                if variant == 'other_path':
                    (root/'copy.md').write_bytes(data)
                    self.commit(root)
                with patch('tools.audit_source_tree.REVIEWED_ATTRIBUTION_LABELS', approved):
                    findings = scan(root)['findings']
                flags = {flag for finding in findings for flag in finding['flags']}
                if variant == 'reviewed':
                    self.assertEqual(findings, [])
                elif variant == 'secret':
                    self.assertIn('github_token', flags)
                else:
                    self.assertIn('decompiler_label', flags)

    def test_gallery_exception_is_exact_and_does_not_hide_changed_or_aliased_binaries(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.initialize(root)
            blob = b'RIFF\x00\x00\x00\x00WEBP synthetic display fixture'
            digest = hashlib.sha256(blob).hexdigest()
            folder = root/'docs/gallery'
            (folder/'media').mkdir(parents=True)
            name = f'media/{digest}.webp'
            image = folder/name
            image.write_bytes(blob)
            (folder/'manifest.json').write_text(json.dumps({'images': {name: digest}}), encoding='utf8')
            self.commit(root)
            self.assertEqual(scan(root)['findings'], [])
            (root/'unapproved.webp').write_bytes(blob)
            image.write_bytes(blob+b'changed')
            self.commit(root)
            findings = scan(root)['findings']
            self.assertIn('unapproved.webp', [f['path'] for f in findings])
            flags = {flag for f in findings for flag in f['flags']}
            self.assertIn('approved_image_hash_or_format_mismatch', flags)
            self.assertIn('binary_nul', flags)
