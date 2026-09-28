"""Offline audit of archived outputs; does not query models or regenerate data."""
from pathlib import Path
import csv,json,math,collections
import numpy as np
ROOT=Path(__file__).resolve().parent
base=ROOT/'acsac2026-paper/experiments/ex1_risk_taxonomy/results'
raw=json.loads((base/'raw_results.json').read_text())
summary=json.loads((base/'summary_stats.json').read_text())
valid=[r for r in raw if r['judge_label']!='error']
assert len(raw)==summary['n_total']==3600
assert len(valid)==summary['n_valid']==2989
cells=list(csv.DictReader((base/'cell_errors.csv').open()))
for c in cells:
 rows=[r for r in raw if r['model']==c['model'] and r['category']==c['category']]
 assert len(rows)==int(c['total'])
 for label in ('elicited','partial','refused'):
  assert sum(r['judge_label']==label for r in rows)==int(c[label])
assert sum(int(c['http_error']) for c in cells)==605
assert sum(int(c['judge_error']) for c in cells)==6

def verify(rows,mean,interval):
 a=np.array([r['judge_label']=='elicited' for r in rows],dtype=float)
 samples=np.random.default_rng(42).choice(a,size=(2000,len(a)),replace=True).mean(axis=1)
 assert math.isclose(float(a.mean()),mean,abs_tol=1e-14)
 assert np.allclose(np.percentile(samples,[2.5,97.5]),interval,rtol=0,atol=1e-14)
verify(valid,summary['overall_elicitation_rate'],summary['overall_ci'])
for r in summary['per_category']:verify([x for x in valid if x['category']==r['category']],r['mean'],[r['ci_lo'],r['ci_hi']])
for r in summary['per_model']:verify([x for x in valid if x['model']==r['model']],r['elicitation_rate'],[r['ci_lo'],r['ci_hi']])
k=json.loads((base/'kappa.json').read_text())
pairs=[json.loads(s) for s in (base/'kappa_pairs.jsonl').read_text().splitlines() if s]
pairs=[r for r in pairs if r['primary_label']!='error' and r['secondary_label']!='error']
assert len(pairs)==k['n']==359
pobs=sum(r['primary_label']==r['secondary_label'] for r in pairs)/len(pairs)
a=collections.Counter(r['primary_label'] for r in pairs);b=collections.Counter(r['secondary_label'] for r in pairs)
pexp=sum(a[c]*b[c] for c in a)/len(pairs)**2
assert math.isclose((pobs-pexp)/(1-pexp),k['kappa'],abs_tol=1e-14)
print('PASS: all 60 recorded cells, 18 sets of bootstrap estimates, and 359-pair kappa')
