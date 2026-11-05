import hashlib
import json
from pathlib import Path
import struct
import tempfile
import unittest
from unittest.mock import patch

from tools import ps2_symbol_lookup as lookup


def synthetic_elf():
    data = bytearray(512)
    ident = b'\x7fELF\x01\x01' + bytes(10)
    struct.pack_into('<16sHHIIIIIHHHHHH', data, 0,
                     ident, 2, 8, 1, 0, 0, 64, 0, 52, 0, 0, 40, 3, 0)
    strings = b'\0ordinary__Fv\0source_file\0'
    data[256:256 + len(strings)] = strings
    struct.pack_into('<IIIIIIIIII', data, 104, 0, 3, 0, 0, 256, len(strings), 0, 0, 1, 0)
    struct.pack_into('<IIIIIIIIII', data, 144, 0, 2, 0, 0, 320, 48, 1, 0, 4, 16)
    struct.pack_into('<IIIBBH', data, 336, 1, 0x100000, 32, 0x12, 0, 1)
    struct.pack_into('<IIIBBH', data, 352, 15, 0, 0, 4, 0, 0xfff1)
    return bytes(data)


class SymbolLookupTests(unittest.TestCase):
    def test_only_literal_function_symbols_are_candidates(self):
        self.assertEqual(lookup.functions(synthetic_elf()), [dict(
            address='0x00100000', raw_symbol='ordinary__Fv', size=32)])
        with self.assertRaises(ValueError):
            lookup.functions(b'not an ELF input')

    def test_wrong_prototype_cannot_acquire_a_trusted_identity(self):
        with self.assertRaisesRegex(ValueError, 'digest mismatch'):
            lookup.worksheet(synthetic_elf(), b'synthetic PC', 0x401000,
                             ['ordinary'], 'module::function')

    def test_lookup_never_promotes_matches_or_demangled_labels(self):
        elf = synthetic_elf()
        csv = ('addr,size,kind,demangled,mangled\n'
               '0x00100000,32,FUNC,ordinary(void),ordinary__Fv\n')
        with patch.object(lookup, 'PROTOTYPE_SHA256', hashlib.sha256(elf).hexdigest()):
            report = lookup.worksheet(elf, b'synthetic PC', 0x401000,
                                      ['ORDINARY'], 'module::function', csv)
            missing = lookup.worksheet(elf, b'synthetic PC', 0x401000,
                                       ['unmatched'], 'module::function', csv)
        self.assertEqual(report['pc']['aliases'], [f'FUN_{0x401000:08x}'])
        self.assertEqual(report['pc']['sha256'], hashlib.sha256(b'synthetic PC').hexdigest())
        self.assertEqual(report['candidates'][0]['demangled'], 'ordinary(void)')
        self.assertEqual(report['candidates'][0]['demangle_status'], 'unreviewed')
        self.assertEqual(report['attribution_status'], 'unknown')
        self.assertFalse(report['publication_reviewed'])
        self.assertEqual(missing['candidates'], [])
        self.assertTrue(missing['lookup']['completed'])

    def test_csv_cannot_substitute_a_raw_symbol_from_another_binary(self):
        elf = synthetic_elf()
        csv = ('addr,size,kind,demangled,mangled\n'
               '0x00100000,32,FUNC,ordinary(void),invented__Fv\n')
        with patch.object(lookup, 'PROTOTYPE_SHA256', hashlib.sha256(elf).hexdigest()):
            with self.assertRaisesRegex(ValueError, 'absent from this ELF'):
                lookup.worksheet(elf, b'PC', 0x401000, ['ordinary'], 'module::function', csv)

    def test_cli_keeps_inputs_read_only_and_worksheets_private_and_fresh(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            elf, pc = root / 'prototype', root / 'pc'
            elf.write_bytes(synthetic_elf())
            pc.write_bytes(b'synthetic PC')
            output = root / '.local' / 'record.json'
            args = ['lookup', '--elf', str(elf), '--pc-binary', str(pc),
                    '--pc-address', '00401000', '--query', 'ordinary',
                    '--rust', 'module::function', '--output', str(output)]
            with (patch.object(lookup, 'ROOT', root),
                  patch.object(lookup, 'PROTOTYPE_SHA256', hashlib.sha256(elf.read_bytes()).hexdigest()),
                  patch('sys.argv', args), patch('builtins.print')):
                lookup.main()
                self.assertEqual(json.loads(output.read_text())['attribution_status'], 'unknown')
                with self.assertRaises(FileExistsError):
                    lookup.main()
                args[-1] = str(root / 'public.json')
                with self.assertRaisesRegex(ValueError, 'private'):
                    lookup.main()
                args[-1] = str(elf)
                with self.assertRaisesRegex(ValueError, 'private'):
                    lookup.main()
            self.assertEqual(elf.read_bytes(), synthetic_elf())
            self.assertEqual(pc.read_bytes(), b'synthetic PC')
