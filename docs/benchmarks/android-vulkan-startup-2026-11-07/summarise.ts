import { resolve } from 'node:path';
const median = (values: number[]) => { const a = values.toSorted((a, b) => a - b); return a.length % 2 ? a[a.length >> 1] : (a[a.length / 2 - 1] + a[a.length / 2]) / 2; };
for (const directory of process.argv.slice(2)) {
  const data = await Bun.file(resolve(directory, 'samples.json')).json();
  const before = data.samples.filter((s: any) => s.variant === 'before');
  const after = data.samples.filter((s: any) => s.variant === 'after');
  if (before.length < 15 || after.length < 15) throw Error('At least 15 samples per variant are required');
  const m = (samples: any[], field: string) => median(samples.map(s => field.split('.').reduce((v, k) => v?.[k], s)).filter(Number.isFinite));
  console.log(`\n${data.applicationId}, ${data.serial}: ${before.length}/${after.length} samples; ${data.discards.length} discards\n`);
  console.log('| Metric | Device | Before | After | Change |\n| --- | --- | ---: | ---: | ---: |');
  for (const [label, field] of [['TotalTime', 'totalTimeMs'], ['JavaScript app ready (UI drain)', 'javascriptReadyMs'], ['First content presented', 'firstContentMs']]) {
    const a = m(before, field), b = m(after, field);
    console.log(`| ${label} | ${data.serial} | ${a.toFixed(0)} ms | ${b.toFixed(0)} ms | ${(b - a).toFixed(0)} ms (${((b / a - 1) * 100).toFixed(1)}%) |`);
  }
  console.log('\n| Phase | Before duration | After duration | Before start | After start |\n| --- | ---: | ---: | ---: | ---: |');
  for (const name of ['instance', 'device', 'pipelines', 'swapchain', 'atlas', 'renderer_prepare']) {
    const format = (v: number, digits = 2) => Number.isFinite(v) ? `${v.toFixed(digits)} ms` : '—';
    console.log(`| ${name} | ${format(m(before, 'spans.' + name))} | ${format(m(after, 'spans.' + name))} | ${format(m(before, 'spanStarts.' + name), 0)} | ${format(m(after, 'spanStarts.' + name), 0)} |`);
  }
  console.log('\n| Event | Before | After |\n| --- | ---: | ---: |');
  for (const field of ['firstJavascriptWakeMs', 'javascriptReadyMs', 'events.first_commit_applied', 'events.first_frame_submitted', 'firstContentMs']) console.log(`| ${field} | ${m(before, field).toFixed(0)} ms | ${m(after, field).toFixed(0)} ms |`);
  const feedback = data.samples.flatMap((s: any) => s.pipelineFeedback);
  console.log('\nPipeline feedback:', Object.fromEntries(['hit', 'miss', 'unknown'].map(cache => [cache, feedback.filter((f: any) => f.cache === cache).length])));
}
