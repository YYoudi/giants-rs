from pathlib import Path
import json
import tempfile
import unittest

from tools.cinematic_inventory import manifest, parse, public_summary, write_reports


HEADER = 'cinematic_consumer_audit schema_version=2 cpu_only=true app_created=false gpu_initialized=false audio_initialized=false\n'
END = 'cinematic_consumer_audit finished=true gpu_initialized=false audio_initialized=false scene_complete=false\n'
DECLARATIONS = ('cinematic_audit_declarations actors=2 active_effect_requests=1 voice_requests=1 '
                'subtitle_calls=1 sync_requests=0 numeric_sound_requests=0 music_commands=0 '
                'credits_commands=0 opaque_blocks=0 runtime_profile_supported=true\n'
                'cinematic_audit_stream_resources checked=1 missing=0 scope=file_presence_only\n')


def case(world='Synthetic', sequence='sample', extra=''):
    return f'cinematic_consumer_audit_case world={world} sequence={sequence}\n'+DECLARATIONS+extra


def result(world='Synthetic', sequence='sample'):
    return f'prepared\t{world}\t{sequence}\tall_actor_consumers\tno_gpu_no_audio_no_scene_fidelity\n'


class CinematicInventoryTests(unittest.TestCase):
    def test_final_consumer_route_does_not_count_as_missing_geometry(self):
        extra = ('cinematic_preview_omission attachment_slot=4 class=900\n'
                 'cinematic_common_attachment_request common_created=false common_drawn=false\n'
                 'cinematic_rigid_attachment_deferred actor_slot=4 reason=existing_specialized_consumer\n'
                 'cinematic_rigid_attachment_prepared actor_slot=5 parts=1\n')
        rows = parse(HEADER+case(extra=extra)+END, result(), [('Synthetic', 'sample')])
        self.assertEqual(rows[0]['findings'], [])
        self.assertEqual(rows[0]['routed_elsewhere'], 1)
        self.assertEqual(rows[0]['state'], 'no_reported_gap_unverified')
        self.assertIn('complete_visual_fidelity', rows[0]['unverified'])

    def test_deduplicates_repeated_preparation_diagnostics_without_merging_sources(self):
        signal = 'cinematic_effect_inspection_deferred offset=8 reason="program refused"\n'
        log = HEADER+case(extra=signal*4)+case(sequence='second', extra=signal*3)+END
        rows = parse(log, result()+result(sequence='second'), [('Synthetic', 'sample'), ('Synthetic', 'second')])
        summary = public_summary(rows, {'source_commit': 'a'*40, 'executable_sha256': 'b'*64})
        self.assertEqual(summary['priorities'][0]['affected_sources'], 2)
        self.assertEqual(summary['priorities'][0]['diagnostic_units'], 2)

    def test_distinguishes_partial_preview_and_unverified_live_tracer(self):
        extra = ('cinematic_effect_inspection_deferred reason="visual objects prepared; contacts incomplete"\n'
                 'cinematic_emitted_tracer_pending tracer_backend_ready=false\n')
        row = parse(HEADER+case(extra=extra)+END, result(), [('Synthetic', 'sample')])[0]
        self.assertEqual({f['cause'] for f in row['findings']}, {'effect_partial', 'tracer_review'})

    def test_catches_downstream_commands_not_only_the_initial_frame(self):
        log = (HEADER+case()+END).replace('numeric_sound_requests=0 music_commands=0',
                                         'numeric_sound_requests=3 music_commands=2')
        row = parse(log, result(), [('Synthetic', 'sample')])[0]
        self.assertEqual({f['cause']: f['count'] for f in row['findings']}, {'numeric_audio': 3, 'music': 2})

    def test_rejects_truncation_missing_results_and_repeated_cases(self):
        for log, table in [(HEADER+case(), result()), (HEADER+case()+END, ''),
                           (HEADER+case()+case()+END, result()), (HEADER+case()+END, result()*2)]:
            with self.assertRaises(ValueError):
                parse(log, table, [('Synthetic', 'sample')])

    def test_public_projection_cannot_export_original_labels_paths_or_diagnostics(self):
        row = parse(HEADER+case()+END, result(), [('Synthetic', 'sample')])[0]
        row['sequence'] = 'PRIVATE_SOURCE_LABEL'
        row['world'] = 'PRIVATE_WORLD'
        row['findings'].append(dict(cause='effect_refused', count=1,
                                    diagnostic='C:/PRIVATE_RESEARCH/<script>secret</script>'))
        identity = {'source_commit': 'a'*40, 'executable_sha256': 'b'*64, 'private': 'SECRET'}
        projection = json.dumps(public_summary([row], identity))
        for text in ['PRIVATE', 'SECRET', '<script>', 'C:/']:
            self.assertNotIn(text, projection)
        with tempfile.TemporaryDirectory() as temp:
            write_reports(Path(temp), [row], identity)
            page = (Path(temp)/'index.html').read_text(encoding='utf8')
            self.assertNotIn('<script>secret', page)
            self.assertIn('&lt;script&gt;secret', page)

    def test_manifest_requires_exact_installed_source_coverage(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root/'Bin').mkdir()
            (root/'Bin/flk_one.bin').touch()
            (root/'Bin/flk_two.bin').touch()
            path = root/'cases.tsv'
            path.write_text('Synthetic\tone\nSynthetic\ttwo\n', encoding='utf8')
            self.assertEqual(len(manifest(path, root, 2)), 2)
            path.write_text('Synthetic\tone\nSynthetic\tone\n', encoding='utf8')
            with self.assertRaisesRegex(ValueError, 'Duplicate'):
                manifest(path, root, 2)
            path.write_text('Synthetic\tone\n', encoding='utf8')
            with self.assertRaisesRegex(ValueError, 'cover'):
                manifest(path, root, 1)
