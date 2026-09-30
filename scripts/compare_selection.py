"""Compare all ray hits for the reference's valid block metadata states."""
from pathlib import Path
import math
import sys

def read(path):
    result={}
    for line in Path(path).read_text(encoding='utf-8-sig').splitlines():
        p=line.split()
        if p and p[0]=='selectionray':
            if len(p)!=6:raise ValueError('Malformed selection trace')
            key=tuple(map(int,p[1:5]))
            if key in result:raise ValueError('Duplicate ray observation')
            value=None
            if p[5]!='-':
                distance,face=p[5].split(',');value=(float(distance),int(face))
                if not math.isfinite(value[0]) or value[1] not in range(6):raise ValueError('Invalid ray hit')
            result[key]=value
    return result

if __name__=='__main__':
    rust,java=read(sys.argv[1]),read(sys.argv[2])
    if not rust or not java:sys.exit('Empty selection traces cannot verify compatibility')
    differences=[]
    for key,b in java.items():
        a=rust.get(key)
        if key not in rust or (a is None)!=(b is None) or (a is not None and (abs(a[0]-b[0])>1e-7 or a[1]!=b[1])):
            differences.append(key)
    for key in sorted(differences)[:12]:print(key,rust.get(key),java.get(key))
    print(f'compared={len(java)} differing={len(differences)}')
    sys.exit(bool(differences))
