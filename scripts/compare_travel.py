"""Compare every tick and every position/velocity coordinate from local travel oracles."""
from pathlib import Path
import sys
import math

def read(path):
    result={}
    for line in Path(path).read_text(encoding='utf-8-sig').splitlines():
        p=line.split()
        if p and p[0]=='travel':
            if len(p) != 10 or p[9] not in ('true', 'false'):
                raise ValueError(f'Malformed travel record: {line}')
            key = (p[1], int(p[2]))
            values = [float(v) for v in p[3:9]]
            if key in result or not all(math.isfinite(v) for v in values):
                raise ValueError(f'Duplicate or non-finite travel record: {line}')
            result[key] = (values, p[9])
    return result
if __name__=='__main__':
    rust,java=read(sys.argv[1]),read(sys.argv[2])
    if not rust or not java:sys.exit('Empty traces cannot verify compatibility')
    failures={}
    for key in rust.keys()|java.keys():
        if key not in rust or key not in java:failures[key]='missing';continue
        a,b=rust[key],java[key]
        if a[1]!=b[1] or any(abs(x-y)>2e-7 for x,y in zip(a[0],b[0])):failures[key]=f'rust={a} java={b}'
    for name in sorted({key[0] for key in failures}):
        key=min(key for key in failures if key[0]==name)
        print(f'{key}: {failures[key]}')
    print(f'compared={len(java)} differing={len(failures)}')
    sys.exit(bool(failures))
