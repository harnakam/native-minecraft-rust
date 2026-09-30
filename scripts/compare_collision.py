"""Compare full unions of axis-aligned collision boxes from local behavioral dumps."""
import itertools
import sys
import math
from pathlib import Path

def read(path):
    shapes = {}
    for line in Path(path).read_text(encoding='utf-8-sig').splitlines():
        parts = line.split()
        if parts and parts[0] == 'shape':
            if len(parts) < 3:
                raise ValueError(f'Malformed shape record: {line}')
            key = (int(parts[1]),int(parts[2]))
            boxes = [tuple(map(float, part.split(','))) for part in parts[3:]]
            if key in shapes or any(len(box) != 6 or not all(math.isfinite(v) for v in box)
                                    or any(box[i] > box[i+3] for i in range(3)) for box in boxes):
                raise ValueError(f'Duplicate or invalid shape record: {line}')
            shapes[key] = boxes
    return shapes

def same(a,b):
    if a == b:
        return True
    points = []
    for axis in range(3):
        edges = sorted({box[axis] for box in a+b} | {box[axis+3] for box in a+b})
        points.append([(x+y)/2 for x,y in zip(edges,edges[1:]) if y-x > 1e-9])
    contains = lambda boxes,p: any(all(box[i]<p[i]<box[i+3] for i in range(3)) for box in boxes)
    return all(contains(a,p)==contains(b,p) for p in itertools.product(*points))

if __name__ == '__main__':
    rust,java=read(sys.argv[1]),read(sys.argv[2])
    if not rust or not java:
        sys.exit('Empty dumps cannot verify compatibility')
    differences = [key for key in java if key not in rust or not same(rust[key],java[key])]
    for id in sorted({id for id,meta in differences}):
        print(f'block={id} metadata={[meta for block,meta in differences if block==id]}')
    print(f'compared={len(java)} differing={len(differences)}')
    sys.exit(bool(differences))
