import { cp, mkdir, symlink } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
const root = resolve(process.cwd());
if (!await Bun.file(resolve(root, 'scripts/ink')).exists()) throw Error('Run from an Ink SDK checkout');
const [destination] = process.argv.slice(2);
if (!destination) throw Error('Usage: bun build-probes.ts OUTPUT_DIRECTORY');
const output = resolve(destination);
const work = resolve(root, '.ink/vulkan-startup-probes');
await mkdir(output, { recursive: true });
await mkdir(work, { recursive: true });
const password = 'ink-vulkan-probe-only';
async function run(argv: string[], capture = false) {
  const p = Bun.spawn(argv, { cwd: root, env: { ...process.env, INK_SDK_ROOT: root, INK_KEYSTORE_PASSWORD: password, INK_KEY_PASSWORD: password, INK_PRESENTATION_TIMING: '1' }, stdout: capture ? 'pipe' : 'inherit', stderr: capture ? 'pipe' : 'inherit' });
  const out = capture ? await new Response(p.stdout).text() : '';
  const err = capture ? await new Response(p.stderr).text() : '';
  if (await p.exited) throw Error(`${argv[0]} failed: ${out}${err}`);
  return out;
}
const keystore = resolve(process.env.INK_VULKAN_PROBE_KEYSTORE ?? resolve(output, '../probe.jks'));
await mkdir(dirname(keystore), { recursive: true });
if (!await Bun.file(keystore).exists()) await run(['keytool', '-genkeypair', '-keystore', keystore, '-storepass', password, '-keypass', password, '-alias', 'probe', '-keyalg', 'RSA', '-validity', '30', '-dname', 'CN=Ink Vulkan probe', '-noprompt']);
const cert = (await run(['keytool', '-list', '-v', '-keystore', keystore, '-storepass', password], true)).match(/SHA256:\s*([A-F0-9:]+)/)?.[1];
if (!cert) throw Error('Missing certificate fingerprint');
for (const [source, name] of [['counter', 'counter'], ['light-template', 'template']]) {
  const app = resolve(work, name);
  await cp(resolve(root, 'examples', source), app, { recursive: true, filter: p => !p.split('/').some(x => ['node_modules', '.ink', 'dist'].includes(x)) });
  try { await symlink(resolve(root, 'examples', source, 'node_modules'), resolve(app, 'node_modules')); } catch (e) { if ((e as NodeJS.ErrnoException).code !== 'EEXIST') throw e; }
  let config = await Bun.file(resolve(app, 'ink.toml')).text();
  config = config.replace(/^package = .+$/m, `package = "com.vandam.ink.vulkanprobe.${name}"`);
  config += `\n[signing]\nkeystore = ${JSON.stringify(keystore)}\nkey_alias = "probe"\ncertificate_sha256 = "${cert}"\n`;
  await Bun.write(resolve(app, 'ink.toml'), config);
  if (name === 'template') {
    await mkdir(resolve(app, 'android/src/main'), { recursive: true });
    await Bun.write(resolve(app, 'android/src/main/AndroidManifest.xml'), `<manifest xmlns:android="http://schemas.android.com/apk/res/android">
  <uses-permission android:name="android.permission.ACCESS_WIFI_STATE" />
  <application>
    <service android:name="org.unifiedpush.android.connector.internal.RaiseToForegroundService" android:exported="true" />
    <receiver android:name="org.unifiedpush.android.connector.internal.MessagingReceiverImpl" android:exported="true" />
  </application>
</manifest>\n`);
  }
  await run([resolve(root, 'scripts/ink'), '-C', app, 'build']);
  await cp(resolve(app, '.ink/build/android/outputs/apk/release/app-release.apk'), resolve(output, `${name}.apk`));
}
