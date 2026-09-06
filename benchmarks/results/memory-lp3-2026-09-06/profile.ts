import { writeFileSync } from 'node:fs';
const variant = process.argv[2];
const root = '/Users/vandam/Developer/ink/benchmarks/results/memory-lp3-2026-09-06';
function adb(...args: string[]) { const p = Bun.spawnSync(['adb','-s','LP3LHMA531900140',...args]); if(p.exitCode) throw Error(p.stderr.toString()); return p.stdout.toString(); }
for (let round=1; round<=3; round++) {
 for(const fixture of round%2 ? ['counter','scroll'] : ['scroll','counter']) {
  const pkg = `com.vandam.benchmark.ink.${fixture}`;
  adb('shell','input','keyevent','224');
  adb('shell','am','force-stop',pkg);
  const launch = adb('shell','am','start','-W','-n',`${pkg}/com.vandam.ink.MainActivity`);
  await Bun.sleep(2000);
  const mem = adb('shell','dumpsys','meminfo',pkg);
  writeFileSync(`${root}/${variant}-${fixture}-${round}.txt`,launch+'\n'+mem);
  console.log(variant,fixture,round,mem.match(/TOTAL PSS:.*$/m)?.[0]);
  adb('shell','am','force-stop',pkg);
 }
}
