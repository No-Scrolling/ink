import hashlib, json, subprocess
from pathlib import Path
root = Path('/Users/vandam/Developer/ink')
out = root / 'benchmarks/results/fresh-general-round3-2026-09-29'
paths = subprocess.check_output(['git', 'ls-files', '--cached', '--others', '--exclude-standard', '-z'], cwd=root).decode().split('\0')
excluded = ('benchmarks/results/', '.agent-tools/', 'node_modules/', 'target/', '.ink/')
files = {}
for name in sorted(set(paths)):
    if not name or name.startswith(excluded):
        continue
    path = root / name
    if path.is_file():
        files[name] = {'sha256': hashlib.sha256(path.read_bytes()).hexdigest(), 'bytes': path.stat().st_size}
digest = hashlib.sha256(json.dumps(files, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
result = {'head': subprocess.check_output(['git','rev-parse','HEAD'],cwd=root).decode().strip(), 'combined_sha256':digest, 'excluded_prefixes':excluded, 'files':files}
(out/'source-provenance.json').write_text(json.dumps(result, indent=2)+'\n')
(out/'current-status.txt').write_bytes(subprocess.check_output(['git','status','--short'],cwd=root))
print(json.dumps({'files':len(files),'combined_sha256':digest}))
