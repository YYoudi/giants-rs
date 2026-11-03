"""Build an asset-free preview ZIP; unresolved distribution reviews require --draft."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def sha256(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def dependency_notices(metadata):
    packages = {p['id']: p for p in metadata['packages']}
    nodes = {n['id']: n for n in metadata['resolve']['nodes']}
    root = next(p['id'] for p in packages.values() if p['name'] == 'giants-app')
    seen = set()

    def walk(identity):
        if identity in seen:
            return
        seen.add(identity)
        for dep in nodes[identity]['deps']:
            if any(k['kind'] is None for k in dep['dep_kinds']):
                walk(dep['pkg'])

    walk(root)
    notices, inventory, missing = [], [], []
    for package in sorted((packages[i] for i in seen if packages[i].get('source')), key=lambda p: (p['name'], p['version'])):
        name = f"{package['name']} {package['version']}"
        directory = Path(package['manifest_path']).parent
        candidates = [p for p in directory.iterdir() if p.is_file() and p.resolve().is_relative_to(directory.resolve()) and
                      p.name.upper().startswith(('LICENSE', 'LICENCE', 'COPYING', 'NOTICE', 'COPYRIGHT', 'UNLICENSE'))]
        if package.get('license_file'):
            candidate = (directory/package['license_file']).resolve()
            if candidate.is_relative_to(directory.resolve()) and candidate.is_file():
                candidates.append(candidate)
        # Some published crates omit workspace-level notices. Supplement only
        # from exact-version upstream receipts committed with this repository.
        supplement = ROOT/'third_party/notice-sources.json'
        if not candidates and supplement.is_file():
            for record in json.loads(supplement.read_text(encoding='utf8')):
                if (record['name'], record['version'], record['license']) != (package['name'], package['version'], package.get('license')):
                    continue
                for entry in record['files']:
                    candidate = (ROOT/entry['file']).resolve()
                    allowed = (ROOT/'third_party/notices').resolve()
                    if not candidate.is_relative_to(allowed) or sha256(candidate) != entry['sha256']:
                        raise ValueError('Invalid supplemental dependency notice receipt')
                    candidates.append(candidate)
        files = sorted(set(candidates))
        inventory.append(dict(name=package['name'], version=package['version'], license=package.get('license'),
                              authors=package.get('authors', []), repository=package.get('repository'),
                              notice_files=[p.name for p in files]))
        notices += [f'## {name}', f"License expression: {package.get('license') or 'UNDECLARED'}",
                    'Authors: '+', '.join(package.get('authors', []))]
        if not files or not package.get('license'):
            missing.append(name)
            notices.append('DISTRIBUTION REVIEW REQUIRED: license text or metadata is incomplete.')
        for path in files:
            notices += [f'### {path.name}', path.read_text(encoding='utf8', errors='strict')]
    return '\n\n'.join(notices)+'\n', inventory, missing


def package(executable, receipt, metadata, output, notes, draft=False, project_license=None):
    executable, output = Path(executable), Path(output)
    if output.exists():
        raise ValueError('Refusing overwrite')
    if executable.name != 'giants-app.exe' or executable.read_bytes()[:2] != b'MZ':
        raise ValueError('Expected the independent Windows giants-app.exe')
    if not re.fullmatch(r'[a-f0-9]{40}', receipt.get('source_commit', '')):
        raise ValueError('A full source commit is required')
    if receipt.get('executable_sha256') != sha256(executable):
        raise ValueError('Executable differs from build receipt')
    if receipt.get('target') != 'x86_64-pc-windows-gnu' or not receipt.get('toolchain'):
        raise ValueError('Expected a recorded GNU Windows build')
    third_party, inventory, missing = dependency_notices(metadata)
    blockers = ['Human source/provenance and release review is pending.']
    toolchain_sources = ROOT/'third_party/toolchain-notice-sources.json'
    if toolchain_sources.is_file():
        third_party += '\n# Compiler and linker runtime notices — applicability review pending\n'
        for entry in json.loads(toolchain_sources.read_text(encoding='utf8')):
            if entry['status'] != 'retrieved_review_pending':
                continue
            path = (ROOT/entry['file']).resolve()
            if not path.is_relative_to((ROOT/'third_party/toolchain').resolve()) or sha256(path) != entry['sha256']:
                raise ValueError('Invalid toolchain dependency notice receipt')
            third_party += '\n## '+entry['file']+'\n\n'+path.read_text(encoding='utf8')+'\n'
    blockers.append('Compiler/linker runtime notice applicability review remains pending.')
    if missing:
        blockers.append('Dependency notice review required: '+', '.join(missing))
    if project_license is None:
        blockers.append('The owner has not selected the project source license.')
    # This tool cannot grant legal clearance. Publication requires a separate review.
    if not draft:
        raise ValueError('Only draft packaging is currently enabled; resolve the public-release audit first')
    files = {'giants-app.exe': executable.read_bytes(),
             'NOTICE.md': (ROOT/'NOTICE.md').read_bytes(),
             'Play.ps1': (ROOT/'tools/Play.ps1').read_bytes(),
             'QUICK_START.md': (ROOT/'docs/QUICK_START.md').read_bytes(),
             'RELEASE_NOTES.md': notes.encode('utf8'),
             'THIRD_PARTY_NOTICES.md': third_party.encode('utf8'),
             'dependency-inventory.json': (json.dumps(inventory, ensure_ascii=False, indent=2)+'\n').encode('utf8')}
    if project_license:
        files['LICENSE'] = Path(project_license).read_bytes()
    public_receipt = {k: receipt[k] for k in ('source_commit', 'executable_sha256', 'target', 'toolchain')}
    public_receipt['linker'] = receipt.get('linker', 'unrecorded')
    manifest = dict(schema_version=1, **public_receipt, draft=True, cleared_for_publication=False,
                    distribution_blockers=blockers,
                    files={name: hashlib.sha256(data).hexdigest() for name, data in files.items()})
    files['manifest.json'] = (json.dumps(manifest, indent=2)+'\n').encode('utf8')
    files['SHA256SUMS'] = ''.join(f'{hashlib.sha256(data).hexdigest()}  {name}\n' for name, data in sorted(files.items())).encode('utf8')
    output.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(output, 'x', compression=zipfile.ZIP_DEFLATED, compresslevel=6) as archive:
        for name, data in files.items():
            archive.writestr(name, data)
    output.with_suffix(output.suffix+'.sha256').write_text(f'{sha256(output)}  {output.name}\n', encoding='utf8')
    return manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--executable', type=Path, required=True)
    parser.add_argument('--receipt', type=Path, required=True)
    parser.add_argument('--metadata', type=Path, required=True)
    parser.add_argument('--notes', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--draft', action='store_true')
    parser.add_argument('--project-license', type=Path)
    args = parser.parse_args()
    report = package(args.executable, json.loads(args.receipt.read_text(encoding='utf8')),
                     json.loads(args.metadata.read_text(encoding='utf8')), args.output,
                     args.notes.read_text(encoding='utf8'), args.draft, args.project_license)
    print(json.dumps(dict(output=str(args.output), cleared_for_publication=False,
                         distribution_blockers=report['distribution_blockers'])))


if __name__ == '__main__':
    main()
