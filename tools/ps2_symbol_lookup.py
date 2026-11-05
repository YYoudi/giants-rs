"""Read-only name lookup; write an untrusted attribution worksheet under .local."""
import argparse
import csv
import hashlib
import json
from pathlib import Path
import struct

ROOT = Path(__file__).resolve().parents[1]
PROTOTYPE_SHA256 = 'be17fa4b1b5c5120dd685891894c93e65509e55bed59756110f39b0ea48258ef'


def functions(data):
    """Read ELF32 little-endian MIPS symbol facts, without executing the target."""
    if data[:6] != b'\x7fELF\x01\x01':
        raise ValueError('Expected ELF32 little-endian input')
    header = struct.unpack_from('<16sHHIIIIIHHHHHH', data)
    if header[2] != 8 or header[11] != 40:
        raise ValueError('Expected MIPS ELF section headers')
    sections = [struct.unpack_from('<IIIIIIIIII', data, header[6] + i * 40)
                for i in range(header[12])]
    result = []
    for section in sections:
        if section[1] != 2:  # SHT_SYMTAB
            continue
        if section[9] != 16:
            raise ValueError('Unexpected ELF symbol size')
        strings = sections[section[6]]
        table = data[strings[4]:strings[4] + strings[5]]
        for at in range(section[4], section[4] + section[5], 16):
            name, address, size, info, _, index = struct.unpack_from('<IIIBBH', data, at)
            if name and info & 15 == 2 and index:
                raw = table[name:table.index(b'\0', name)].decode('utf8')
                result.append(dict(address=f'0x{address:08x}', raw_symbol=raw, size=size))
    return result


def worksheet(elf, pc, address, queries, rust, symbol_csv=None):
    digest = hashlib.sha256(elf).hexdigest()
    if digest != PROTOTYPE_SHA256:
        raise ValueError('Prototype digest mismatch; do not mix binary identities')
    symbols = functions(elf)
    labels = {}
    if symbol_csv is not None:
        facts = {(s['address'], s['raw_symbol'], s['size']) for s in symbols}
        for row in csv.DictReader(symbol_csv.splitlines()):
            if row['kind'] != 'FUNC':
                continue
            key = (f"0x{int(row['addr'], 16):08x}", row['mangled'], int(row['size']))
            if key not in facts:
                raise ValueError('CSV function is absent from this ELF')
            labels[key[:2]] = row['demangled']
    candidates = []
    for symbol in symbols:
        label = labels.get((symbol['address'], symbol['raw_symbol']))
        if any(q.casefold() in (symbol['raw_symbol'] + ' ' + (label or '')).casefold()
               for q in queries):
            candidates.append(symbol | dict(demangled=label, demangle_status='unreviewed'))
    return dict(schema_version=1, pc=dict(sha256=hashlib.sha256(pc).hexdigest(),
                address=f'0x{address:08x}', aliases=[f'FUN_{address:08x}']),
                prototype=dict(sha256=digest), lookup=dict(queries=queries, completed=True),
                rust=rust, attribution_status='unknown', candidates=candidates,
                evidence=[], divergences=[], publication_reviewed=False)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--elf', type=Path, required=True)
    parser.add_argument('--pc-binary', type=Path, required=True)
    parser.add_argument('--pc-address', type=lambda v: int(v, 16), required=True)
    parser.add_argument('--query', action='append', required=True)
    parser.add_argument('--rust', required=True)
    parser.add_argument('--symbol-csv', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    if (not output.is_relative_to(ROOT / '.local')
            or any(output == p.resolve() for p in (args.elf, args.pc_binary, args.symbol_csv) if p)):
        raise ValueError('Worksheet output must be fresh, private and separate from inputs')
    report = worksheet(args.elf.read_bytes(), args.pc_binary.read_bytes(), args.pc_address,
                       args.query, args.rust,
                       args.symbol_csv.read_text(encoding='utf8') if args.symbol_csv else None)
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open('x', encoding='utf8') as stream:
        json.dump(report, stream, indent=2)
        stream.write('\n')
    print(json.dumps(dict(candidates=len(report['candidates']), attribution_status='unknown',
                          publication_reviewed=False)))


if __name__ == '__main__':
    main()
