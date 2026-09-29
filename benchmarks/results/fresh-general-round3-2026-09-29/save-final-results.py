import json, os, subprocess
from pathlib import Path
root=Path('/Users/vandam/Developer/ink')
out=root/'benchmarks/results/fresh-general-round3-2026-09-29'
env=dict(os.environ, INK_AGENT_TOOLS_DIR=str(out/'tools'))
ids=['headless-20260929-212344-0103308a','headless-20260929-212711-ccda1b28','headless-20260929-212726-1d010cfc','headless-20260929-212959-1566cc79','experiment-20260929-213033-2edcdae0','headless-20260929-213200-128e9264','experiment-20260929-213200-56a99e03','headless-20260929-213358-cccdb9a8','headless-20260929-214432-6c8220ea']
results=[json.loads(subprocess.check_output([str(root/'scripts/agent-tools'),'result',i],cwd=root,env=env)) for i in ids]
(out/'final-operations.json').write_text(json.dumps(results,indent=2)+'\n')
print(json.dumps({r['id']:r['status'] for r in results}))
