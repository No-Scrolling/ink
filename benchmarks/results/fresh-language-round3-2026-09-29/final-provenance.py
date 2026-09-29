"""Hash current source inputs without reading previous review outputs."""
import hashlib
import json
import subprocess
from datetime import datetime, timezone
from pathlib import Path

root = Path('/Users/vandam/Developer/ink')
out = root / 'benchmarks/results/fresh-language-round3-2026-09-29'
raw = subprocess.check_output(['git', 'ls-files', '-z', '--cached', '--others', '--exclude-standard'], cwd=root)
paths = sorted(set(raw.decode().split('\0')) - {''})
hashes = {}
for name in paths:
    if name.startswith('benchmarks/results/') or '.agent-tools' in Path(name).parts:
        continue
    path = root / name
    if path.is_file():
        hashes[name] = hashlib.sha256(path.read_bytes()).hexdigest()
initial = json.loads((out / 'provenance.json').read_text())['paths']
snapshot = {
    'capturedUTC': datetime.now(timezone.utc).isoformat(),
    'head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root).decode().strip(),
    'paths': hashes,
    'changedSinceInitial': {p: {'initial': initial.get(p), 'final': hashes.get(p)}
        for p in sorted(initial.keys() | hashes.keys()) if initial.get(p) != hashes.get(p)},
}
evidence = ['proxy-final-quickjs.ts', 'proxy-final-bundle/proxy-final-quickjs.js', 'quickjs-driver/Cargo.toml',
    'quickjs-driver/Cargo.lock', 'quickjs-driver/src/main.rs', 'proxy-final-quickjs.json',
    'capture-depth-quickjs.ts', 'capture-depth-quickjs.json', 'capture-depth-quickjs-release.json',
    'capture-depth-quickjs-ink-dev.json', 'compiler-depth-runtime-builder.ts',
    'compiled-capture-depth-runtime.tsx', 'compiled-capture-depth-bundle/compiled-capture-depth-runtime.js',
    'compiled-capture-depth-dev.json', 'compiled-capture-depth-release.json', 'compiled-capture-depth-ink-dev.json',
    'function-property-runtime-builder.ts', 'function-property-initial-source.tsx',
    'function-property-initial-runtime.tsx', 'function-property-initial-bundle.js',
    'function-property-initial-quickjs.json', 'function-property-source.tsx', 'function-property-runtime.tsx',
    'function-property-bundle/function-property-runtime.js', 'function-property-final-quickjs.json',
    'compiled-capture-depth-final-release.json', 'compiled-capture-depth-final-ink-dev.json']
snapshot['diagnosticArtefacts'] = {p: hashlib.sha256((out / p).read_bytes()).hexdigest() for p in evidence}
snapshot['diagnosticToolchain'] = {
    'rust': subprocess.check_output(['rustc', '--version'], cwd=root).decode().strip(),
    'bun': subprocess.check_output(['bun', '--version'], cwd=root).decode().strip(),
}
(out / 'final-provenance.json').write_text(json.dumps(snapshot, indent=2) + '\n')
print(json.dumps({'sourceInputs': len(hashes), 'changedSinceInitial': snapshot['changedSinceInitial'],
    'diagnosticArtefacts': snapshot['diagnosticArtefacts']}, indent=2))
