"""Scan reachable Git history for selected publication hazards, without printing values.

This technical inventory does not establish provenance or legal clearance.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess

PATTERNS = {
    'personal_path': re.compile(rb'(?i)[a-z]:[\\/]Users[\\/]|[\\/]home[\\/][a-z][a-z0-9_-]*[\\/]'),
    'private_key': re.compile(rb'-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----'),
    'github_token': re.compile(rb'\b(?:ghp_[a-zA-Z0-9]{30,}|github_pat_[a-zA-Z0-9_]{40,})\b'),
    'openai_token': re.compile(rb'\bsk-(?:proj-)?[a-zA-Z0-9_-]{35,}\b'),
    'decompiler_label': re.compile(rb'\b(?:FUN_|DAT_|sub_)[0-9a-fA-F]{6,}\b'),
}
RESTRICTED = {'.exe', '.dll', '.gzp', '.bin', '.dmp', '.zip', '.png', '.jpg',
              '.jpeg', '.webp', '.wav', '.ogg', '.mp3', '.ttf', '.otf', '.flk', '.gti', '.gb2'}

# Reviewed minimal PC aliases, not a general allowance for analysis labels.
# Bind the exception to the exact Git blob bytes and its sole public path.
# Other hazard patterns remain active even for this reviewed document.
REVIEWED_ATTRIBUTION_LABELS = {
    'docs/MECC_DESCENT_GUARD.md': {
        'ec1186bdfc4d82c4a070d6fb6caf83008aa7d97d7446f86f4eb6744a9dd1c75b',
        '878226a742b54851495e1c4c8de62951ab030502d55c460d685fba6c669d0c3c',
        '314abf495905a93b662e78674834cac06439378ced9e33b4e1bc856eb2b96760',
        'ec1186bdfc4d82c4a070d6fb6caf83008aa7d97d7446f86f4eb6744a9dd1c75b',
    },
    'docs/RECONSTRUCTION_METHOD.md': {
        # Reviewed history-scanner workflow prose; existing minimal aliases only.
        '0fc5c80adf51d81f0270c44ba86ddacb7728cc84cef0b6aed6d33ba21aca002c',
        # Two minimal unassigned PC aliases; explicit missing helper/counterpart evidence.

        # Historical source-lane variant: two additional bounded, explicitly
        # unassigned PC/Rust aliases; no corpus, native body or new PS2 claim.
        '47c224fb1d9581ee570e6399c8a868cf779824a58936242b8b2a4a23d06c2ee5',
        # RNG ownership prose only; the reviewed alias list is unchanged.
        '0cb7e8d16f16186d5552729a1ec077b453a8f0dfea3de16858d1c966edbf6af5',
        '2c8a49007d040af5c353ce6921860566a63b61fbb67c95c2a57b7da479567e0d',
        '07638b2e77f9a2eeb7bd63e55e91c9c0e72f5643dcf536571733f74397b3d94f',
        '28763ac079e17b02931cc074f17ae85238f9d9d7f87f69d1251d176bbf719b1a',
        'b95db50f67716523141c0bf3d11ccf1928ea2299c48f39fea32501ee09f0c44f',
    },
}


def approved_images(root):
    """Exact display copies authorized by the owner; no general binary exception."""
    manifest = Path(root)/'docs/gallery/manifest.json'
    if not manifest.exists():
        return {}
    images = json.loads(manifest.read_text(encoding='utf8')).get('images', {})
    return {'docs/gallery/'+name: digest for name, digest in images.items()
            if re.fullmatch(r'media/[a-f0-9]{64}\.webp', name)
            and Path(name).stem == digest}


def scan(root):
    approved = approved_images(root)
    def git(*args):
        return subprocess.check_output(['git', '-C', str(root), *args])

    commits = git('rev-list', '--all').decode().splitlines()
    blobs, restricted = {}, set()
    # Inspect every pathname in every reachable commit, including historical aliases.
    for commit in commits:
        for row in git('ls-tree', '-rz', commit).split(b'\0'):
            if not row:
                continue
            fields, raw_path = row.split(b'\t', 1)
            mode, kind, identity = fields.decode().split()
            path = raw_path.decode('utf8')
            if (Path(path).suffix.lower() in RESTRICTED and path not in approved) or mode in ('120000', '160000'):
                restricted.add(path)
            if kind == 'blob':
                blobs.setdefault(identity, set()).add(path)
    findings = [dict(path=p, flags=['restricted_path_or_link']) for p in sorted(restricted)]
    with subprocess.Popen(['git', '-C', str(root), 'cat-file', '--batch'],
                          stdin=subprocess.PIPE, stdout=subprocess.PIPE) as reader:
        for identity, paths in blobs.items():
            path = sorted(paths)[0]
            reader.stdin.write((identity+'\n').encode())
            reader.stdin.flush()
            header = reader.stdout.readline().decode().split()
            data = reader.stdout.read(int(header[2]))
            reader.stdout.read(1)
            digest = hashlib.sha256(data).hexdigest()
            reviewed_labels = all(digest in REVIEWED_ATTRIBUTION_LABELS.get(p, ())
                                  for p in paths)
            flags = [name for name, pattern in PATTERNS.items()
                     if pattern.search(data)
                     and not (name == 'decompiler_label' and reviewed_labels)]
            approved_image = (all(p in approved for p in paths)
                              and all(hashlib.sha256(data).hexdigest() == approved[p] for p in paths)
                              and data.startswith(b'RIFF') and data[8:12] == b'WEBP')
            if any(p in approved for p in paths) and not approved_image:
                flags.append('approved_image_hash_or_format_mismatch')
            if b'\0' in data and not approved_image:
                flags.append('binary_nul')
            if flags:
                findings.append(dict(path=path, blob=identity, flags=flags))
        reader.stdin.close()
        reader.wait()
    return dict(head=git('rev-parse', 'HEAD').decode().strip(),
                reachable_commits=len(commits), unique_blobs_scanned=len(blobs),
                findings=findings,
                limitations='Selected patterns only; no protected-expression comparison or legal clearance.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    report = scan(Path(__file__).resolve().parents[1])
    with args.output.open('x', encoding='utf8') as output:
        json.dump(report, output, indent=2)
        output.write('\n')
    print(json.dumps({k: report[k] for k in ('head', 'reachable_commits', 'unique_blobs_scanned')}
                     | dict(findings=len(report['findings']))))
    if report['findings']:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
