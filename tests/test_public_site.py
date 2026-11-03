import json
from pathlib import Path
import tempfile
import unittest
from tools.export_site import export, project


class PublicSiteTests(unittest.TestCase):
    def fixture(self):
        return {'secret': 'DO_NOT_EXPORT', 'private_annex': 'PRIVATE', 'readme_roadmap': [
            {'id': 'mission', 'kind': 'feature', 'group': 'Campaigns', 'title': '<script>alert(1)</script>',
             'status': 'partial', 'scope': 'Runtime', 'detail': 'Objectives', 'private': 'DO_NOT_EXPORT'}]}

    def test_projection_drops_private_fields_and_refuses_paths_in_public_fields(self):
        source = self.fixture()
        self.assertNotIn('DO_NOT_EXPORT', json.dumps(project(source)))
        source['readme_roadmap'][0]['detail'] = 'evidence/private/report.json'
        with self.assertRaisesRegex(ValueError, 'Private path'):
            project(source)

    def test_site_escapes_data_and_exports_only_explicit_files(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root/'PROGRESS.json').write_text(json.dumps(self.fixture()), encoding='utf8')
            (root/'private.dump').write_bytes(b'NEVER EXPORT')
            output = root/'site'
            export(output, root)
            self.assertEqual({p.name for p in output.iterdir()}, {'index.html', 'app.js', 'status.json', '.nojekyll'})
            text = (output/'index.html').read_text(encoding='utf8')
            self.assertIn('&lt;script&gt;', text)
            self.assertNotIn('<script>alert(1)', text)
            self.assertNotIn('DO_NOT_EXPORT', text)
            with self.assertRaisesRegex(ValueError, 'overwrite'):
                export(output, root)
