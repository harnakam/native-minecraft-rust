"""Translate observed numeric block bounds into independent selection lookup rules."""
from collections import defaultdict
from pathlib import Path
import math
import sys

def ranges(values):
    result=[]
    values=sorted(values)
    start=previous=values[0]
    for value in values[1:]:
        if value==previous+1:previous=value;continue
        result.append(str(start) if start==previous else f'{start}..={previous}')
        start=previous=value
    result.append(str(start) if start==previous else f'{start}..={previous}')
    return ' | '.join(result)

def main():
    groups=defaultdict(list)
    seen=set()
    for line in Path(sys.argv[1]).read_text(encoding='utf-8-sig').splitlines():
        p=line.split()
        if p and p[0]=='select':
            state=int(p[1])*16+int(p[2])
            if state in seen:sys.exit('Duplicate selection observation')
            seen.add(state)
            if p[3]=='false':continue
            box=tuple(map(float,p[4].split(',')))
            if len(box)!=6 or not all(math.isfinite(value) for value in box):sys.exit('Invalid bounds')
            groups[box].append(state)
    if len(seen)!=3039:sys.exit('Incomplete selection observations')
    lines=['//! Numeric selection bounds observed using authored local MCP probes.',
           'use crate::collision::Aabb;', 'pub fn bounds(state: u16) -> Option<Aabb> {', '    match state {']
    for box,states in sorted(groups.items()):
        lines.append(f'        {ranges(states)} => Some(Aabb::new({list(box[:3])!r}, {list(box[3:])!r})),')
    lines.extend(['        _ => None,', '    }', '}'])
    Path(sys.argv[2]).write_text('\n'.join(lines)+'\n',encoding='utf-8')

if __name__=='__main__':main()
