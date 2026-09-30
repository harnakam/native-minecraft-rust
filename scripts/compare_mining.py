"""Compare every mining observation by its exact IEEE single-precision value."""
from pathlib import Path
import math
import struct
import sys

def read(path):
    result={}
    for line in Path(path).read_text(encoding='utf-8-sig').splitlines():
        p=line.split()
        if p and p[0]=='hardness':
            if len(p)!=5:raise ValueError('Malformed mining record')
            key=(p[1],int(p[2]),int(p[3]))
            value=float(p[4])
            if key in result or math.isnan(value):raise ValueError('Duplicate or NaN mining observation')
            result[key]=struct.pack('!f',value)
    return result

if __name__=='__main__':
    rust,java=read(sys.argv[1]),read(sys.argv[2])
    if not rust or not java:sys.exit('Empty mining traces cannot verify compatibility')
    differences=[key for key in rust.keys()|java.keys() if rust.get(key)!=java.get(key)]
    for key in sorted(differences)[:12]:print(key)
    print(f'compared={len(java)} differing={len(differences)}')
    sys.exit(bool(differences))
