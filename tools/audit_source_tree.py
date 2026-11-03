"""Scan reachable Git history for selected publication hazards, without printing values.

This technical inventory does not establish provenance or legal clearance.
"""
import argparse
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
              '.jpeg', '.wav', '.ogg', '.mp3', '.ttf', '.otf', '.flk', '.gti', '.gb2'}


def scan(root):
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
            if Path(path).suffix.lower() in RESTRICTED or mode in ('120000', '160000'):
                restricted.add(path)
            if kind == 'blob':
                blobs.setdefault(identity, path)
    findings = [dict(path=p, flags=['restricted_path_or_link']) for p in sorted(restricted)]
    with subprocess.Popen(['git', '-C', str(root), 'cat-file', '--batch'],
                          stdin=subprocess.PIPE, stdout=subprocess.PIPE) as reader:
        for identity, path in blobs.items():
            reader.stdin.write((identity+'\n').encode())
            reader.stdin.flush()
            header = reader.stdout.readline().decode().split()
            data = reader.stdout.read(int(header[2]))
            reader.stdout.read(1)
            flags = [name for name, pattern in PATTERNS.items() if pattern.search(data)]
            if b'\0' in data:
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
