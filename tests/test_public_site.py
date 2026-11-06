import json
from pathlib import Path
import tempfile
import unittest
from tools.export_site import export, project


class PublicSiteTests(unittest.TestCase):
    def test_cinematic_inventory_exports_only_canonical_aggregate_fields(self):
        from tools.cinematic_inventory import public_summary
        from tools.export_site import cinematic_inventory
        row = dict(world='PRIVATE_WORLD', sequence='PRIVATE_SOURCE', preparation='prepared',
                   state='reported_gap', findings=[dict(cause='numeric_audio', count=3,
                                                       diagnostic='PRIVATE_REPORT')])
        data = public_summary([row], {'source_commit': 'a'*40, 'executable_sha256': 'b'*64})
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root/'docs').mkdir()
            path = root/'docs/cinematic-inventory-summary.json'
            def save():
                path.write_text(json.dumps(data), encoding='utf8')
            save()
            exported, section = cinematic_inventory(root)
            self.assertIn('Cinematic gaps to prioritize', section)
            self.assertNotIn('PRIVATE', json.dumps(exported)+section)
            data['private_sources'] = ['PRIVATE_SOURCE']
            save()
            with self.assertRaisesRegex(ValueError, 'fields'):
                cinematic_inventory(root)
            del data['private_sources']
            data['priorities'][0]['label'] = 'evidence/private/report'
            save()
            with self.assertRaisesRegex(ValueError, 'Noncanonical'):
                cinematic_inventory(root)
            data['priorities'][0]['label'] = public_summary([row], {
                'source_commit': 'a'*40, 'executable_sha256': 'b'*64})['priorities'][0]['label']
            data['visual_fidelity_checked'] = True
            save()
            with self.assertRaisesRegex(ValueError, 'verification claim'):
                cinematic_inventory(root)

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

    def test_featured_capture_is_static_escaped_and_requires_a_reviewed_row(self):
        import hashlib
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root/'PROGRESS.json').write_text(json.dumps(self.fixture()), encoding='utf8')
            folder = root/'docs/gallery'
            (folder/'media').mkdir(parents=True)
            blob = b'synthetic display fixture'
            digest = hashlib.sha256(blob).hexdigest()
            name = f'media/{digest}.webp'
            (folder/name).write_bytes(blob)
            title = '<script>untrusted title</script>'
            data = {'featured_comparisons': [title], 'comparisons': [
                dict(title=title, left=name, right=name, right_label='Engine "view"')],
                'images': {name: digest}}
            manifest = folder/'manifest.json'
            manifest.write_text(json.dumps(data), encoding='utf8')
            export(root/'site', root)
            text = (root/'site/index.html').read_text(encoding='utf8')
            self.assertIn('id="highlights"', text)
            self.assertLess(text.index('id="highlights"'), text.index('id="overview"'))
            self.assertIn(f'<img src="{name}"', text)
            self.assertIn('&lt;script&gt;untrusted title&lt;/script&gt;', text)
            self.assertIn('alt="Engine &quot;view&quot;"', text)
            self.assertNotIn(title, text)
            data['featured_comparisons'] = ['Unreviewed selection']
            manifest.write_text(json.dumps(data), encoding='utf8')
            with self.assertRaisesRegex(ValueError, 'reviewed gallery row'):
                export(root/'invalid-site', root)
