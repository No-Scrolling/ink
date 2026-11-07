import sys,json,zipfile,hashlib
from pathlib import Path
label=sys.argv[1]
result={}
for name in ('counter','template','inkify'):
 p=Path(f'.size-work/{name}-{label}.apk')
 if not p.exists():continue
 with zipfile.ZipFile(p) as z:
  entries={i.filename:{'bytes':i.file_size,'compressed_bytes':i.compress_size,'sha256':hashlib.sha256(z.read(i)).hexdigest()} for i in z.infolist()}
  row={'apk':p.stat().st_size,'native_library':z.getinfo('lib/arm64-v8a/libink_android.so').file_size,'native_libraries_total':sum(i.file_size for i in z.infolist() if i.filename.endswith('.so')),'dex':sum(i.file_size for i in z.infolist() if i.filename.endswith('.dex')),'resources.arsc':z.getinfo('resources.arsc').file_size,'app.jsbc':entries.get('assets/app.jsbc',{}).get('bytes',0),'worker_bytecode':entries.get('assets/worker.jsbc',{}).get('bytes',0),'ink-web':entries.get('assets/ink-assets/ink-web.jsbc',{}).get('bytes',0)}
  result[name]={'sizes':row,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'entries':entries}
output=Path('docs/benchmarks/android-size-2026-11-07')/f'{label}.json'
output.write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({k:v['sizes'] for k,v in result.items()},indent=2))
