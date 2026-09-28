import json, subprocess, statistics
from pathlib import Path
root=Path(__file__).resolve().parent
agents=root.parent
builds={'O3':'headless-20260928-234558-668ec124','Os':'headless-20260928-234626-7996bdaf','Oz':'headless-20260928-234708-492ff1df'}
fixtures={'React updates':('headless-20260928-234558-668ec124',[]),'Native bindings':('headless-20260928-233412-ea281923',[]),'Lists':('headless-20260928-233458-e929f186',['--list-compat']),'Native controls':('headless-20260928-233458-39a325ff',['--native-controls'])}
records=[]
for fixture,(assets,flags) in fixtures.items():
 for iteration in range(6):
  order=['O3','Os','Oz']
  shift=iteration%3
  order=order[shift:]+order[:shift]
  if iteration>=3: order.reverse()
  for mode in order:
   cmd=[str(agents/builds[mode]/'ink-headless'),'--assets',str(agents/assets/'assets'),'--iterations','200','--warmup','20',*flags]
   result=json.loads(subprocess.check_output(cmd))
   result.pop('inputToSceneSamplesMs',None)
   records.append({'fixture':fixture,'mode':mode,'round':iteration,'result':result})
   (root/'paired-cpu.json').write_text(json.dumps(records,indent=2))
 for mode in builds:
  rs=[r['result'] for r in records if r['fixture']==fixture and r['mode']==mode]
  print(fixture,mode,{k:round(statistics.mean(r['milliseconds'][k]['mean'] for r in rs), 4) for k in rs[0]['milliseconds']},flush=True)
