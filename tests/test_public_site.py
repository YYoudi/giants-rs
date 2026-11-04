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
            self.assertEqual({p.name for p in output.iterdir()}, {'index.html', 'app.js', 'style.css', 'status.json', 'gallery.json', '.nojekyll'})
            text = (output/'index.html').read_text(encoding='utf8')
            self.assertIn('&lt;script&gt;', text)
            self.assertNotIn('<script>alert(1)', text)
            self.assertNotIn('DO_NOT_EXPORT', text)
            with self.assertRaisesRegex(ValueError, 'overwrite'):
                export(output, root)

    def test_gallery_rejects_private_fields_traversal_and_changed_images(self):
        import hashlib
        from tools.export_site import gallery_data
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            folder = root/'docs/gallery'
            folder.mkdir(parents=True)
            blob = b'synthetic fixture'
            digest = hashlib.sha256(blob).hexdigest()
            name = f'media/{digest}.webp'
            (folder/'media').mkdir()
            (folder/name).write_bytes(blob)
            data = {'comparisons': [{'left': name, 'right': name, 'title': 'Synthetic'}], 'images': {name: digest}}
            def save():
                (folder/'manifest.json').write_text(json.dumps(data), encoding='utf8')
            save()
            self.assertEqual(len(gallery_data(root)['comparisons']), 1)
            data['comparisons'][0]['manifest'] = 'private report'
            save()
            with self.assertRaisesRegex(ValueError, 'Unexpected gallery field'):
                gallery_data(root)
            del data['comparisons'][0]['manifest']
            data['comparisons'][0]['left'] = '../private.webp'
            save()
            with self.assertRaisesRegex(ValueError, 'Unapproved gallery image'):
                gallery_data(root)
            data['comparisons'][0]['left'] = name
            save()
            (folder/name).write_bytes(b'changed')
            with self.assertRaisesRegex(ValueError, 'hash mismatch'):
                gallery_data(root)
