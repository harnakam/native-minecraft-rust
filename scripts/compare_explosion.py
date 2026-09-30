"""Compare exact S27 bytes and decoded block positions from authored MCP probes."""
from pathlib import Path
import sys

def read(path):
    records={}
    for line in Path(path).read_text(encoding='utf-8-sig').splitlines():
        p=line.split()
        if p and p[0]=='explosion':
            if len(p)!=4 or p[1] not in {'0','1','2','3'} or p[2] not in {'wire','pos0','pos1'}:
                raise ValueError('Malformed explosion record')
            key=(p[1],p[2])
            if key in records:raise ValueError('Duplicate explosion record')
            if p[2]=='wire':
                if len(bytes.fromhex(p[3]))!=39:raise ValueError('Invalid explosion wire length')
            elif len(list(map(int,p[3].split(','))))!=3:raise ValueError('Invalid position')
            records[key]=p[3]
    if len(records)!=12:raise ValueError('Incomplete explosion trace')
    return records

if __name__=='__main__':
    rust,java=read(sys.argv[1]),read(sys.argv[2])
    differences=[key for key in java if rust.get(key)!=java[key]]
    for key in differences:print(key,rust.get(key),java[key])
    print(f'compared={len(java)} differing={len(differences)}')
    sys.exit(bool(differences))
