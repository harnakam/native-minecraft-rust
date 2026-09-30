"""Compare controller progress, activity, predicted world state and every digging action."""
from pathlib import Path
import math
import struct
import sys

def read(path):
    result={}
    for line in Path(path).read_text(encoding='utf-8-sig').splitlines():
        p=line.split()
        if p and p[0]=='minestate':
            if len(p)!=7 or p[4] not in ('true','false') or p[5] not in ('true','false'):
                raise ValueError('Malformed controller trace')
            key=(p[1],int(p[2]))
            damage=float(p[3])
            if key in result or not math.isfinite(damage):raise ValueError('Duplicate or invalid controller progress')
            result[key]=(struct.pack('!f',damage),p[4],p[5],p[6])
    return result

if __name__=='__main__':
    rust,java=read(sys.argv[1]),read(sys.argv[2])
    if not rust or not java:sys.exit('Empty controller traces cannot verify compatibility')
    differences=[key for key in rust.keys()|java.keys() if rust.get(key)!=java.get(key)]
    for key in sorted(differences)[:12]:print(key,rust.get(key),java.get(key))
    print(f'compared={len(java)} differing={len(differences)}')
    sys.exit(bool(differences))
