"""Verify a preview ZIP without extracting or executing its contents.

Integrity checks do not establish provenance or distribution clearance.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import zipfile


def verify(archive_path):
    archive_path = Path(archive_path)
    digest = hashlib.sha256(archive_path.read_bytes()).hexdigest()
    expected = f'{digest}  {archive_path.name}\n'
    if archive_path.with_suffix(archive_path.suffix+'.sha256').read_text(encoding='utf8') != expected:
        raise ValueError('Archive checksum mismatch')
    required = {'giants-app.exe', 'Play.ps1', 'NOTICE.md', 'QUICK_START.md',
                'RELEASE_NOTES.md', 'THIRD_PARTY_NOTICES.md', 'dependency-inventory.json',
                'manifest.json', 'SHA256SUMS'}
    with zipfile.ZipFile(archive_path) as archive:
        names = archive.namelist()
        members = set(names)
        if len(names) != len(members) or not required <= members or members - required - {'LICENSE'}:
            raise ValueError('Unexpected, duplicate or missing archive members')
        contents = {name: archive.read(name) for name in names}
    manifest = json.loads(contents['manifest.json'])
    if (manifest.get('schema_version') != 1 or type(manifest.get('draft')) is not bool or
            manifest.get('cleared_for_publication') is not (not manifest['draft']) or
            not isinstance(manifest.get('distribution_blockers'), list) or
            not all(isinstance(b, str) and b.strip() for b in manifest['distribution_blockers'])):
        raise ValueError('Inconsistent preview distribution state')
    if not manifest['draft']:
        review = manifest.get('distribution_review', {})
        keys = {'authorization', 'dependency_notices_sha256', 'toolchain_notices_sha256',
                'review_document_sha256', 'project_license_sha256'}
        if (manifest['distribution_blockers'] or 'LICENSE' not in members or set(review) != keys or
                review.get('authorization') != 'owner_authorized_technical_review' or
                any(not re.fullmatch(r'[a-f0-9]{64}', review.get(k, '')) for k in keys-{'authorization'}) or
                review['project_license_sha256'] != hashlib.sha256(contents['LICENSE']).hexdigest()):
            raise ValueError('Public preview lacks a consistent distribution review')
    if (not re.fullmatch(r'[a-f0-9]{40}', manifest.get('source_commit', '')) or
            manifest.get('target') != 'x86_64-pc-windows-gnu' or
            not manifest.get('toolchain')):
        raise ValueError('Invalid source or toolchain identity')
    hashed = members - {'manifest.json', 'SHA256SUMS'}
    actual = {name: hashlib.sha256(contents[name]).hexdigest() for name in hashed}
    if manifest.get('files') != actual:
        raise ValueError('Manifest file checksum mismatch')
    if (manifest.get('executable_sha256') != actual['giants-app.exe'] or
            not contents['giants-app.exe'].startswith(b'MZ')):
        raise ValueError('Executable identity mismatch')
    checksums = ''.join(f'{hashlib.sha256(contents[name]).hexdigest()}  {name}\n'
                        for name in sorted(members - {'SHA256SUMS'}))
    if contents['SHA256SUMS'] != checksums.encode('utf8'):
        raise ValueError('Internal checksum list mismatch')
    return dict(source_commit=manifest['source_commit'], archive_sha256=digest,
                executable_sha256=manifest['executable_sha256'], files=len(members),
                cleared_for_publication=manifest['cleared_for_publication'],
                distribution_blockers=manifest['distribution_blockers'])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('archive', type=Path)
    args = parser.parse_args()
    print(json.dumps(verify(args.archive), indent=2))


if __name__ == '__main__':
    main()
