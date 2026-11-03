import json
from pathlib import Path
import tempfile
import unittest
import zipfile
import shutil
import subprocess
from unittest.mock import patch

from tools.package_preview import package, sha256, dependency_notices


class PreviewDistributionTests(unittest.TestCase):
    def test_supplemental_notice_must_match_receipt_and_stay_in_notice_tree(self):
        with tempfile.TemporaryDirectory() as directory:
            root,exe,metadata,receipt=self.fixture(directory)
            (root/'dependency/LICENSE').unlink()
            notice=root/'third_party/notices/synthetic-1/LICENSE'
            notice.parent.mkdir(parents=True)
            notice.write_text('Synthetic permission notice',encoding='utf8')
            entry=dict(file='third_party/notices/synthetic-1/LICENSE',sha256=sha256(notice))
            records=[dict(name='synthetic',version='1',license='MIT',files=[entry])]
            source=root/'third_party/notice-sources.json'
            source.write_text(json.dumps(records),encoding='utf8')
            with patch('tools.package_preview.ROOT',root):
                self.assertEqual(dependency_notices(metadata)[2],[])
                notice.write_text('tampered text',encoding='utf8')
                with self.assertRaisesRegex(ValueError,'notice receipt'):
                    dependency_notices(metadata)
                entry['file']='dependency/Cargo.toml'
                source.write_text(json.dumps(records),encoding='utf8')
                with self.assertRaisesRegex(ValueError,'notice receipt'):
                    dependency_notices(metadata)

    @unittest.skipUnless(shutil.which('pwsh'), 'PowerShell is required')
    def test_launcher_checks_paths_without_running_a_game(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);bundle=root/'preview with spaces';bundle.mkdir()
            script=bundle/'Play.ps1'
            shutil.copyfile(Path(__file__).resolve().parents[1]/'tools/Play.ps1',script)
            (bundle/'giants-app.exe').write_bytes(b'MZ not runnable')
            data=root/'game with spaces';data.mkdir();(data/'Bin').mkdir();(data/'Bin/test.gzp').write_bytes(b'synthetic')
            command=['pwsh','-NoProfile','-File',str(script),'-DataRoot',str(data),'-UserDataRoot',str(root/'user'),'-CheckOnly']
            result=subprocess.run(command,capture_output=True,text=True)
            self.assertEqual(result.returncode,0,result.stderr)
            args=json.loads(result.stdout)
            self.assertIn('--mecc-menu',args)
            self.assertEqual(args[args.index('--data-root')+1],str(data))
            self.assertFalse((root/'user').exists())
            command[command.index('-UserDataRoot')+1]=str(data/'user')
            self.assertNotEqual(subprocess.run(command,capture_output=True).returncode,0)

    def fixture(self, directory):
        root=Path(directory)
        exe=root/'giants-app.exe';exe.write_bytes(b'MZ synthetic test executable')
        dep=root/'dependency';dep.mkdir();(dep/'LICENSE').write_text('Synthetic permission notice',encoding='utf8')
        metadata=dict(packages=[dict(id='app',name='giants-app',version='0',source=None),
                                dict(id='dep',name='synthetic',version='1',source='test',license='MIT',manifest_path=str(dep/'Cargo.toml'))],
                      resolve=dict(nodes=[dict(id='app',deps=[dict(pkg='dep',dep_kinds=[dict(kind=None)])]),dict(id='dep',deps=[])]))
        receipt=dict(source_commit='a'*40,executable_sha256=sha256(exe),target='x86_64-pc-windows-gnu',toolchain='test',private='PRIVATE_PATH')
        return root,exe,metadata,receipt

    def test_draft_has_only_allowlisted_files_hashes_and_no_private_metadata(self):
        with tempfile.TemporaryDirectory() as directory:
            root,exe,metadata,receipt=self.fixture(directory)
            (root/'game.gzp').write_bytes(b'never distribute')
            output=root/'preview.zip'
            report=package(exe,receipt,metadata,output,'Incomplete preview',draft=True)
            self.assertFalse(report['cleared_for_publication'])
            with zipfile.ZipFile(output) as archive:
                self.assertNotIn('game.gzp',archive.namelist())
                self.assertIn('Play.ps1',archive.namelist())
                self.assertIn('Synthetic permission notice',archive.read('THIRD_PARTY_NOTICES.md').decode())
                self.assertNotIn('PRIVATE_PATH',archive.read('manifest.json').decode())
                for name,digest in report['files'].items():
                    import hashlib
                    self.assertEqual(hashlib.sha256(archive.read(name)).hexdigest(),digest)
            with self.assertRaisesRegex(ValueError,'overwrite'):
                package(exe,receipt,metadata,output,'',draft=True)

    def test_tampering_and_uncleared_publication_are_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            root,exe,metadata,receipt=self.fixture(directory)
            with self.assertRaisesRegex(ValueError,'Only draft'):
                package(exe,receipt,metadata,root/'public.zip','')
            exe.write_bytes(b'MZ changed executable')
            with self.assertRaisesRegex(ValueError,'differs'):
                package(exe,receipt,metadata,root/'draft.zip','',draft=True)

    def test_missing_dependency_notices_remain_a_blocker(self):
        with tempfile.TemporaryDirectory() as directory:
            root,exe,metadata,receipt=self.fixture(directory)
            (root/'dependency/LICENSE').unlink()
            report=package(exe,receipt,metadata,root/'draft.zip','',draft=True)
            self.assertTrue(any('synthetic 1' in text for text in report['distribution_blockers']))


if __name__=='__main__':unittest.main()
