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
    # Historical Rust module comments each contain exactly one reviewed PC
    # navigation alias. No native body/listing or corpus extraction is present.
    # Current source keeps addresses and routes full aliases to the minimal
    # navigation document; these pins do not permit future source revisions.
    'crates/giants-game/src/ripper_primary_turn.rs': {
        '5f2656262ec4687a9cbb2aea9bfc4548b7b662b981fceae331014002a4a6f3f1',
        'cc40f87c485effeebf7654b46e36c06c0939c2ddec1c05efa8e469922e5407d8',
    },
    'crates/giants-game/src/ripper_steering.rs': {
        'fd9d4c969e0366239611982359a37c4c8ecb42642c12310dc54bc5462a345dc9',
    },
    'docs/SYMBOL_NAVIGATION.md': {
        # Reviewed unknown mission-failure search route and supplied boundaries.
        'f2776022fa2db7dbb90551368f5867882e5e7855d00c9cf2f46f603a2d7b65e3',
        # Concurrent reviewed source-replacement navigation; explicit unknown.
        '68be833a7ea484145b14dc1a3b98ed0e093bcfa48e0ebee68da7b522e3e259e8',
        # Concurrent reviewed constructed-rigid render route; explicit unknown.
        'f1ddb3f0b5d77a3402f1754c5462c3d40917f4ed233c7836b0adb74c447975fa',
        # Concurrent reviewed guard-effect ledger route; explicit unknown.
        '7656fa6dea7d487005fb3aa7f52ca5eb68e802ceba88e09ad9a454dcc4bacc50',
        # Reviewed literal Mecc logic countdown route; unknown, no raw corpus.
        '4d9c6df516e0d4ae3d05fd233345f2c4eab68042fec4368ae797a25117aafd44',
        # Reviewed canonical cold Mecc hit publication route; still unknown.
        '3c630d00d511a4255f590c5c25d92e95965f23b1858583b5aef2f66916a5c912',
        # Concurrent reviewed no-shot building route; unknown, no raw research.
        'fec0bcb9d32839696060f3d3d3ed4c24ac2d15f49e923a271613cfb9edf3939c',
        # Reviewed direct object-contact route and distinct receipt; unknown.
        '8c9942d8203dea074fa21772306aa572c391baa9ee7793f25b9918df6d023c72',
        # Reviewed source-bearing nonlethal Mecc hit composition, still unknown.
        '3bcb07ec1630417373e0141df1282d84357e28f9e79fc84b1256b72b1d34759e',
        # Concurrent reviewed player-mesh navigation route, explicitly unknown.
        '2c3b3331dfb1987ef99acdc805c98795d167817d25ec23ed2f5522d5313fce71',
        # Concurrent reviewed static-shadow routes, explicitly unknown.
        '03e8861c2ecbe4903cf621c55c5db2fb033667893ec92292a22e8bc778b3851c',
        # Reviewed Mecc hurt search route, explicitly separate from firing.
        '5e82c7c946bcbf67d9a9f6819797f1b7e4fe454a4049572f4fbfb2bc649f7aac',
        # Reviewed mixed population host link; existing unknown identities kept.
        '7708ab21c2a4d160585947c987754d1d3642419e70c281d5fccf1bec57a5ab0e',
        # Reviewed static platform query routes and explicit unknown attribution.
        'b434786fb3d2623c653f9ae6154df0c6665c0853ebf647452bb614fe443900fd',
        # Concurrent reviewed building-damage routes; no raw research included.
        'afb941293abb3d32e8ad9786412ad7313fa74272812fd59b4834c70ecfd8ef2a',
        # Reviewed exact debug renderer identity and unknown gradient route.
        '7318289ec0febb0e42994630d51c5ed4fbaa0a877a19c431087f94a3d29cf26b',
        # Reviewed canonical mixed draw and parameter-consumer route, unknown.
        '44e785ab0473ed1100e6f26d1475252f658dce7de8aba1627cb559e1a911f678',
        # Reviewed canonical mixed construction/wait route; unknown attribution.
        '3a4639af649016925f7e40e0074c93610161ce965d4671457179e584db7afd0d',
        # Reviewed positional impact mode8 entry; unknown wrapper attribution.
        '432dd2cc494e8f0e03ffb55a811f8e0f6bb3b2d99c8c4e748638ba811f02962d',
        # Concurrent reviewed building navigation: names/status only, no corpus.
        'cc1455ed8f9482f0b38033862bac6ad8e6d5c694ee720eea79d7d6df7a289042',
        # Reviewed immediate one-shot placement route; unknown attribution.
        '38f1549bfb93f66e3db9cc5692d28c3e9ce9f125adeec2ab532098768c9165df',
        # Reviewed mixed root constructor link; unknown attribution, no corpus.
        'aaa4e812aeac37c37de759e17eac26c4f6b2a6ff860bc22310b5d554c3619be5',
        # Reviewed canonical scroll/fade parameter route; unknown attributions.
        '55e1e942fb5c0a0c0dea7a15490e1c40c1a497fb4eea09b07a774cf791c44e5d',
        # Reviewed pending scroll/fade navigation; unknown attributions, no corpus.
        '1838eb8ef285d2d6694ae0bad8cf698bce43e4065ad6bbf233e77b50c2bd9164',
        # Reviewed union of the main mission/FX and NPC radial navigation;
        # existing aliases and attribution bounds retained, no corpus/body.
        'b497d47f79eb19dd5b3455524821b5f5f7be528f6b730f569d31510aac582432',
        # Reviewed radial Session link: two literal prototype search names,
        # unknown attribution and unchanged prior aliases; no corpus/listing.
        'ca9102a7b7a15dca9445650493f64ac66ddc0585b274295b388ff873b795ef1f',
        # Concurrent minimal Smartie/Flick navigation additions, unknown
        # attributions and existing aliases only; no native bodies or corpus.
        '649c65e331b31019c5529089c3a134b48ca0f5e7fa38fab353773800a66fce71',
        'ff17acf1136bcda4a7ab43810f0cec702b34b0caee3ffd8c71142484a174ab3d',
        # Reviewed class230 cooldown/retreat source links; attribution stays
        # unknown and the existing minimal PC alias is unchanged.
        '31cb1af7881794447c0af562f2ffed0428747a30fcf9e102caa83128029b72de',
        # Concurrent cinematic navigation: unknown prototype spawn route and
        # explicit support-class divergence; no native body or corpus export.
        '816a22fe27ee50d63ba1112fc44c811dd85a089ca25db342a3c9246d860da5ea',
        # Reviewed launch-bridge cross-reference and unassigned helper address;
        # retained aliases/statuses, no new native body or corpus extraction.
        '01e32dd58e2c6046541c1faeb5280bc7ed1c4f4d4a19d0a29aa639420bc40daf',
        # Concurrent effect-lane navigation: reviewed probable opcode block and
        # explicit prototype-stub/PC-behavior divergence, same minimal PC alias.
        '1a51cdd4703a5d425c901d99cacd504c620b9c398a0cb0dfe8c92c5d6e8b753c',
        # Reviewed literal ripper_shot search candidate and one retained PC alias;
        # explicitly unknown, no native body or detailed private evidence.
        '59317e0dbd9965e3e71770b97a367753556f8b73c1fe6f7efc6c218a915e7cad',
        # Concurrent mission-lane navigation variant: reviewed probable/unknown
        # Smartie, finalization and ground-effect routes; same minimal PC alias.
        'de47ee624eb38830f889233c0eb6288dac84c2afda13239a5c5ff4fc6f81a89e',
        # Reviewed literal mound and common cleanup search candidates, all unknown;
        # retained minimal PC aliases only, with no new native body or corpus.
        '0d55775d69b6004163df1693a6a096fbfaef5ccdebcaa97aa98a9655a3c06369',
        # One independently reviewed, probable mission-proximity PC alias, with
        # the prototype radius/PC AABB divergence and no exported native body.
        '3fdd90bb86ddfe497ae92e08f4555fe7f43d01d0ec58525b52a99bd57aed84d2',
        # Heading and mission-boundary cross-reference update only.
        '225d9984e410f75aacc14d0909df94c3e23186cc55a08501c8b1d0ca2457b4d4',
        # Reviewed minimal Ripper navigation only: literal prototype names,
        # retained PC aliases and explicitly unknown correspondences. No body,
        # corpus rows, signatures or personal provenance are included.
        'b26f4e3f7a7b03952d2eb8ec7372f605a6a41e1cfd0b75dad41a6bd6b348601c',
        'bda631e89d7dc3dcbf4cfdc5367a7496ffc4a8c8e29baf6b40ab931f61af856a',
        # Same reviewed names/aliases; adds the related attack module link.
        'f7e4796f4304094585f3888c7ecc41a3ffaed9feb51ed56be75ce5dc4a464270',
        # Concurrent mission-lane variant: the previous reviewed PC alias is
        # unchanged; adds a bounded, explicitly probable Smartie navigation key.
        '5c74b545f08d84b522c809b3876449bd7efdb542321cb6bfdb4cffc8e718bd12',
        # Adds one literal effect search key and an explicitly unknown PC
        # preparation alias; no effect/native body or corpus record is exported.
        'e5e1bb1d18f316137adb758d97c16f5fb7ad11ae4430292472b48c9e8c6c7a89',
        # Reviewed unknown animation-navigation name/address and owned-caller
        # link only; no new attribution, signature, native body or corpus rows.
        'a1063f01eb56b87bf6ce00a783fddaeb092c1c40bd1f238e7ef5d865424a3917',
        # Adds reviewed unassigned state48 navigation addresses/module link;
        # existing minimal aliases are unchanged, with no native body or table.
        '10095ce35c431a67ebb92a38dfe5657e2ba9f6726a73ee274226d4871a7595bb',
        # Concurrent mission-lane revision: reviewed probable physical-finalizer
        # navigation/policy distinction; existing minimal PC aliases unchanged.
        '73f6c89fb54c84a0d1afde6fecf09b99a8fef464342d94695472a3e9f2c66a94',
    },
    'docs/MECC_DESCENT_GUARD.md': {
        # Gameplay-first work-selection prose only; the reviewed alias is unchanged.
        '8a5e2f5f60606203255e389c489b0ed88ecf69566e77d52321231905e0fc589d',
        'ec1186bdfc4d82c4a070d6fb6caf83008aa7d97d7446f86f4eb6744a9dd1c75b',
        '878226a742b54851495e1c4c8de62951ab030502d55c460d685fba6c669d0c3c',
        '314abf495905a93b662e78674834cac06439378ced9e33b4e1bc856eb2b96760',
        'ec1186bdfc4d82c4a070d6fb6caf83008aa7d97d7446f86f4eb6744a9dd1c75b',
    },
    'docs/RECONSTRUCTION_METHOD.md': {
        # Reviewed module-identity requirement; prior attribution rows unchanged.
        '85ac9cb7e1ddf1256cc141adfa4b823f9328cace3c93b13a0b1c5885c38875de',
        # Observed instruction versus indirect function-entry evidence;
        # existing reviewed attribution aliases remain unchanged.
        '74014108617eae1798deacd3bb76527bbcf437e723b0e7b4634ef7db17268ee2',
        # Central mission-method link only; reviewed attribution aliases unchanged.
        'a0c4addf0a738b4bf910f9f730d85fd8410c09ffed60c7ba683fd7de201a868a',
        # Searchable-source naming policy; existing attribution rows unchanged.
        '484ac3487d9ce9092f5ac771124701aee131802459afbcda2f3edfd414eb7e13',
        # Gameplay-first selection/handoff prose only; attribution rows are unchanged.
        'bbf89e34ec9cc515385bb66664e978ce79f2ba20d1ec47764e936d95a704fc34',
        # Historical exact-blob review guidance; existing minimal alias rows unchanged.
        '0fc5c80adf51d81f0270c44ba86ddacb7728cc84cef0b6aed6d33ba21aca002c',
        # Historical source-lane variant: two additional bounded, explicitly
        # unassigned PC/Rust aliases; no corpus, native body or new PS2 claim.
        '47c224fb1d9581ee570e6399c8a868cf779824a58936242b8b2a4a23d06c2ee5',
        # Same two reviewed, unassigned rows in the newer method revision;
        # the remaining bytes match an already-reviewed document exactly.
        '0dab2114631cdbdee1414f5151d0295450ecee5db66be95f0aa4913a707f0eef',
        # RNG ownership prose only; the reviewed alias list is unchanged.
        '0cb7e8d16f16186d5552729a1ec077b453a8f0dfea3de16858d1c966edbf6af5',
        '2c8a49007d040af5c353ce6921860566a63b61fbb67c95c2a57b7da479567e0d',
        '07638b2e77f9a2eeb7bd63e55e91c9c0e72f5643dcf536571733f74397b3d94f',
        '28763ac079e17b02931cc074f17ae85238f9d9d7f87f69d1251d176bbf719b1a',
        'b95db50f67716523141c0bf3d11ccf1928ea2299c48f39fea32501ee09f0c44f',
    },
}


# Exact display copy pinned in the gallery manifest at 8c385dc, on a public
# branch. Reviewed as a VP8-only image without embedded metadata. Keep its
# all-reference history review valid before that gallery update enters main;
# this permits neither other images nor another path or modified bytes.
REVIEWED_HISTORICAL_IMAGES = {
    # Concurrent .13.2 display copies pinned by the manifest at eafa3e4.
    # Visually reviewed VP8-only images; exact hashes, without embedded metadata.
    'docs/gallery/media/06cf7a33aca7f6605075996727045686d1a03931c3643177e08329adc5633464.webp':
        '06cf7a33aca7f6605075996727045686d1a03931c3643177e08329adc5633464',
    'docs/gallery/media/53d6fc48475f3b3770d82c43232dce7fa8f60b7e2233536e1b55fba50cbd55be.webp':
        '53d6fc48475f3b3770d82c43232dce7fa8f60b7e2233536e1b55fba50cbd55be',
    'docs/gallery/media/cf69f85e72326b3a0f05140bea912776df93d9e607b05f77e75288e988782a29.webp':
        'cf69f85e72326b3a0f05140bea912776df93d9e607b05f77e75288e988782a29',
    # Concurrent .13.1 Bevy display copies pinned by the reviewed manifest.
    # VP8-only, visually reviewed; no research inputs or metadata imported.
    'docs/gallery/media/4607b6cb426cc6c9f043f34a434a009f1680be56e8abaf427d262b5a08591455.webp':
        '4607b6cb426cc6c9f043f34a434a009f1680be56e8abaf427d262b5a08591455',
    'docs/gallery/media/9b97a6efb4a94e05cd5a191f930797e6def06291fcbc9141c11875681efdd42b.webp':
        '9b97a6efb4a94e05cd5a191f930797e6def06291fcbc9141c11875681efdd42b',
    'docs/gallery/media/d26e82e1c36918d3b5090e1a95b09da82f6a7b9f1fc7481d8c6048c35d2dc7f5.webp':
        'd26e82e1c36918d3b5090e1a95b09da82f6a7b9f1fc7481d8c6048c35d2dc7f5',
    # Exact Bevy display copies pinned by the integration manifest at 863fa88.
    # Reviewed visually and as VP8-only files with no embedded metadata. Retain
    # all-reference audit coverage without importing that lane's publication.
    'docs/gallery/media/2e78aa81e7e9cabbabbc86bb4fe412e70bc93e8be8f95d5a3f3da657fb5fbd07.webp':
        '2e78aa81e7e9cabbabbc86bb4fe412e70bc93e8be8f95d5a3f3da657fb5fbd07',
    'docs/gallery/media/65f1882d19ebace68ff15cdf582f67348c07eb1e2964c08aead98a7f03cfb406.webp':
        '65f1882d19ebace68ff15cdf582f67348c07eb1e2964c08aead98a7f03cfb406',
    'docs/gallery/media/0c974350a8c2e0089bd48ed425923f96febe559fafcd8599d01223b0311385e4.webp':
        '0c974350a8c2e0089bd48ed425923f96febe559fafcd8599d01223b0311385e4',
    # Exact Bevy display copies pinned by the integration manifest at 078c8eb.
    # Reviewed visually and as VP8-only files with no embedded metadata. Keep
    # their already-reachable history auditable without importing that lane's
    # gallery/download update. Changed bytes or another path remain refused.
    'docs/gallery/media/82aeb77fb01574d9baac585fa0ad3b4c533afcdd91e40bb445913fe42fae8bfb.webp':
        '82aeb77fb01574d9baac585fa0ad3b4c533afcdd91e40bb445913fe42fae8bfb',
    'docs/gallery/media/8da5780cdb6c29fc0298c8328aa2e8a392d025e5075db41351d9b2c3b1540cf9.webp':
        '8da5780cdb6c29fc0298c8328aa2e8a392d025e5075db41351d9b2c3b1540cf9',
    # Concurrent Bevy display copies pinned by the manifest at d03b4eb.
    # Reviewed visually and as VP8-only files, with no embedded metadata.
    # Preserve the integration lane's work without importing its publication.
    'docs/gallery/media/555c1f8e593741a5903b3af5eeadd23c187d5281945e24460af4219e485b2c63.webp':
        '555c1f8e593741a5903b3af5eeadd23c187d5281945e24460af4219e485b2c63',
    'docs/gallery/media/abb79713e439250d582f4c4994d9638d51eaea633d83bfa5ea31b78d10470425.webp':
        'abb79713e439250d582f4c4994d9638d51eaea633d83bfa5ea31b78d10470425',
    'docs/gallery/media/d4710d638d15205eda7b2d3997b22dd25a93c228b1e88970cddc7cbc91ec7fbc.webp':
        'd4710d638d15205eda7b2d3997b22dd25a93c228b1e88970cddc7cbc91ec7fbc',
}


def approved_images(root):
    """Exact display copies authorized by the owner; no general binary exception."""
    manifest = Path(root)/'docs/gallery/manifest.json'
    if not manifest.exists():
        return dict(REVIEWED_HISTORICAL_IMAGES)
    images = json.loads(manifest.read_text(encoding='utf8')).get('images', {})
    return dict(REVIEWED_HISTORICAL_IMAGES) | {
        'docs/gallery/'+name: digest for name, digest in images.items()
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
