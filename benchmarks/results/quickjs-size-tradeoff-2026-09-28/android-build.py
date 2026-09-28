import os, subprocess, shutil, json
from pathlib import Path
root = Path(__file__).resolve().parent
ndk = Path('/Users/vandam/Library/Android/sdk/ndk/29.0.14206865')
llvm = ndk / 'toolchains/llvm/prebuilt/darwin-x86_64'
env = dict(os.environ, ANDROID_NDK_HOME=str(ndk), CARGO_TARGET_DIR='/Users/vandam/Developer/ink/target', CARGO_PROFILE_RELEASE_STRIP='none')
env['BINDGEN_EXTRA_CLANG_ARGS_aarch64_linux_android'] = f'--sysroot={llvm}/sysroot -I{llvm}/sysroot/usr/include/aarch64-linux-android'
records = []
for mode in ['O3', 'Os', 'Oz']:
    env['CFLAGS_aarch64_linux_android'] = '-' + mode
    command = ['cargo','ndk','-t','arm64-v8a','-p','34','--manifest-path', str(root/'source/Cargo.toml'), 'build','-p','ink-android','--release','--locked']
    with (root / f'{mode}-build.log').open('w') as log:
        subprocess.run(command, cwd=root/'source', env=env, stdout=log, stderr=log, check=True)
    symbols = root / f'{mode}-symbols.so'
    shutil.copy2('/Users/vandam/Developer/ink/target/aarch64-linux-android/release/libink_android.so', symbols)
    stripped = root / f'{mode}.so'
    subprocess.run([str(llvm/'bin/llvm-strip'),'--strip-all','-o',str(stripped),str(symbols)],check=True)
    with (root/f'{mode}-nm.txt').open('w') as log:
        subprocess.run([str(llvm/'bin/llvm-nm'),'--print-size','--size-sort','--demangle',str(symbols)],stdout=log,check=True)
    with (root/f'{mode}-sections.txt').open('w') as log:
        subprocess.run([str(llvm/'bin/llvm-size'),'-A',str(stripped)],stdout=log,check=True)
    record = {'mode':mode,'bytes':stripped.stat().st_size,'command':command,'cflags':env['CFLAGS_aarch64_linux_android']}
    records.append(record)
    (root/'android-sizes.json').write_text(json.dumps(records,indent=2))
    print(mode,record['bytes'],flush=True)
