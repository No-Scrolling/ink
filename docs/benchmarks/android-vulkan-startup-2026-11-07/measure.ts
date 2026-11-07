import { mkdir, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
const [serial, applicationId, before, after, output, count = '15'] = process.argv.slice(2);
if (!output || !/^com\.vandam\.ink\.vulkanprobe\.(counter|template)$/.test(applicationId)) throw Error('Usage: bun measure.ts SERIAL com.vandam.ink.vulkanprobe.counter|template BEFORE_APK AFTER_APK OUTPUT [PAIRS=15]');
await mkdir(output, { recursive: true });
const activity = `${applicationId}/com.vandam.ink.MainActivity`;
async function adb(...args: string[]) {
  const p = Bun.spawn(['adb', '-s', serial, ...args], { stdout: 'pipe', stderr: 'pipe' });
  const [code, out, err] = await Promise.all([p.exited, new Response(p.stdout).text(), new Response(p.stderr).text()]);
  if (code) throw Error(`${args.join(' ')}: ${out}${err}`);
  return out;
}
async function foreground() {
  const state = await adb('shell', 'dumpsys', 'activity', 'activities');
  return state.split('\n').some(l => /mResumedActivity|topResumedActivity/.test(l) && l.includes(applicationId + '/'));
}
const initial = new Set((await adb('shell', 'pm', 'list', 'packages')).trim().split('\n'));
if (initial.has(`package:${applicationId}`)) throw Error('Probe package already installed; refusing to replace it');
const samples: any[] = [], discards: any[] = [];
try {
  for (let pair = 0; pair < Number(count); pair++) for (const variant of ['before', 'after']) {
    await adb('install', '-r', resolve(variant === 'before' ? before : after));
    for (let attempt = 0; ; attempt++) {
      if (attempt >= 10) throw Error('Repeated foreground contention');
      await adb('shell', 'am', 'start', '-W', '-n', activity);
      if (!await foreground()) { discards.push({ pair, variant, stage: 'prepare' }); continue; }
      const token = `vulkan-startup-${Date.now()}-${pair}-${variant}`;
      const launch = await adb('shell', `log -t InkVulkanProbe ${token}; am start -W -S -n ${activity}`);
      const pid = (await adb('shell', 'pidof', applicationId)).trim();
      await Bun.sleep(1200);
      if (!await foreground()) { discards.push({ pair, variant, stage: 'settle' }); continue; }
      const logs = (await adb('logcat', '-d', '-v', 'epoch', '-t', '10000')).split('\n');
      const begin = logs.findLast(l => l.includes(token));
      if (!begin || !/^\d+$/.test(pid)) throw Error('Missing launch marker/PID');
      const start = Number(begin.trim().split(/\s+/)[0]);
      const lines = logs.filter(l => { const f = l.trim().split(/\s+/); return f[1] === pid && Number(f[0]) >= start; });
      const elapsed = (l: string) => Math.round((Number(l.trim().split(/\s+/)[0]) - start) * 1e6) / 1000;
      const wake = lines.find(l => l.includes('Startup javascript_wake'));
      const ready = lines.find(l => l.includes('JavaScript app ready'));
      const firstCommit = lines.findIndex(l => l.includes('Startup first_commit_applied'));
      const presented = lines.find((l, i) => i > firstCommit && firstCommit >= 0 && /Presented id=\d+ scene=[1-9]\d*/.test(l));
      if (!ready || !wake || !presented) throw Error(`Missing startup markers\n${lines.join('\n')}`);
      const spans = Object.fromEntries(lines.flatMap(l => { const m = l.match(/Startup span=(\w+) phase=end duration_ns=(\d+)/); return m ? [[m[1], Number(m[2]) / 1e6]] : []; }));
      const spanStarts = Object.fromEntries(lines.flatMap(l => { const m = l.match(/Startup span=(\w+) phase=begin/); return m ? [[m[1], elapsed(l)]] : []; }));
      const pipelineFeedback = lines.flatMap(l => { const m = l.match(/Startup pipeline=(\w+) cache=(\w+) driver_ns=(\d+)/); return m ? [{ pipeline: m[1], cache: m[2], driverMs: Number(m[3]) / 1e6 }] : []; });
      const events = Object.fromEntries(lines.flatMap(l => { const m = l.match(/Startup (javascript_wake|first_commit_applied|first_frame_submitted)\b/); return m ? [[m[1], elapsed(l)]] : []; }));
      const sample = { pair, variant, totalTimeMs: Number(launch.match(/^TotalTime: (\d+)/m)?.[1]), javascriptReadyMs: elapsed(ready), firstJavascriptWakeMs: elapsed(wake), firstContentMs: elapsed(presented), spans, spanStarts, pipelineFeedback, events, cache: lines.find(l => l.includes('Startup pipeline_cache='))?.split('Startup ')[1] };
      if (!sample.totalTimeMs) throw Error(launch);
      if (pair === 0) {
        if (!await foreground()) throw Error('Lost foreground before screenshot');
        const p = Bun.spawn(['adb', '-s', serial, 'exec-out', 'screencap', '-p'], { stdout: 'pipe' });
        const bytes = await new Response(p.stdout).arrayBuffer();
        if (await p.exited || !await foreground()) throw Error('Screenshot lost foreground');
        await writeFile(resolve(output, `${variant}.png`), new Uint8Array(bytes));
      }
      await writeFile(resolve(output, `${pair}-${variant}.txt`), launch + '\n' + begin + '\n' + lines.join('\n'));
      samples.push(sample);
      await writeFile(resolve(output, 'samples.json'), JSON.stringify({ serial, applicationId, samples, discards }, null, 2));
      console.log(JSON.stringify(sample));
      await adb('shell', 'am', 'force-stop', applicationId);
      break;
    }
  }
} finally {
  const pre = new Set((await adb('shell', 'pm', 'list', 'packages')).trim().split('\n'));
  const result = await adb('uninstall', applicationId);
  const post = new Set((await adb('shell', 'pm', 'list', 'packages')).trim().split('\n'));
  const removed = [...pre].filter(p => !post.has(p));
  const clean = !post.has(`package:${applicationId}`) && removed.length === 1 && removed[0] === `package:${applicationId}`;
  await writeFile(resolve(output, 'cleanup.json'), JSON.stringify({ result: result.trim(), removed, clean }, null, 2));
  if (!clean) throw Error('Probe cleanup/package comparison failed');
}
