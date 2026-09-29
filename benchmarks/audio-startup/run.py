#!/usr/bin/env python3
"""Build and run the isolated Rust/AAudio probe on a reserved Android device."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shlex
import subprocess
import time
import uuid

ROOT = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent

def run(*args, **kwargs):
    return subprocess.run(args, check=True, text=True, **kwargs)

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--serial', required=True)
    parser.add_argument('--token', required=True)
    parser.add_argument('--ndk', type=Path, required=True)
    parser.add_argument('--sharing', choices=['shared', 'exclusive'], default='shared')
    parser.add_argument('--mp3', help='Existing stereo MP3 path on the device; never copied to the host')
    parser.add_argument('--quick', action='store_true', help='Only three warm trials per mode; omit cold/idle measurements')
    args = parser.parse_args()
    status = json.loads(run(str(ROOT/'scripts/agent-tools'), 'device', 'status', '--serial', args.serial,
                            '--full', capture_output=True).stdout)
    if status.get('token') != args.token:
        raise RuntimeError('A matching agent-tools device reservation is required')
    serial = args.serial
    unique = uuid.uuid4().hex[:10]
    output = ROOT/'.agent-tools'/f'aaudio-{unique}'
    output.mkdir(parents=True)
    target = ROOT/'.agent-tools/aaudio/target'
    linker = args.ndk/'toolchains/llvm/prebuilt/darwin-x86_64/bin/aarch64-linux-android28-clang'
    if not linker.is_file():
        raise RuntimeError(f'NDK linker missing: {linker}')
    env = dict(os.environ, CARGO_TARGET_DIR=str(target), CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=str(linker))
    run('cargo', 'build', '--locked', '--manifest-path', str(HERE/'Cargo.toml'),
        '--target', 'aarch64-linux-android', '--release', env=env)
    binary = target/'aarch64-linux-android/release/ink-audio-startup'
    manifest = dict(serial=serial, sharing=args.sharing, real_mp3=bool(args.mp3), started=time.time(),
                    binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                    sources={p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                             for p in HERE.iterdir() if p.suffix in ['.rs', '.toml', '.lock', '.py']})
    (output/'manifest.json').write_text(json.dumps(manifest, indent=2)+'\n')
    remote = '/data/local/tmp/ink-aaudio-'+unique
    command = [remote, args.sharing] + ([args.mp3] if args.mp3 else [])
    if args.quick:
        command.append('--quick')
    try:
        run('adb', '-s', serial, 'push', str(binary), remote)
        with (output/'samples.jsonl').open('w') as stdout, (output/'stderr.log').open('w') as stderr:
            run('adb', '-s', serial, 'shell', shlex.join(command), stdout=stdout, stderr=stderr, timeout=120)
    finally:
        run('adb', '-s', serial, 'shell', 'rm', '-f', remote)
    rows = [json.loads(line) for line in (output/'samples.jsonl').read_text().splitlines()]
    print(json.dumps({'evidence': str(output), 'records': len(rows),
                      'max_xruns': max(r.get('xruns', 0) for r in rows)}))

if __name__ == '__main__':
    main()
