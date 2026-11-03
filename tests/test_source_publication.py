from pathlib import Path
import subprocess
import tempfile
import unittest

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
