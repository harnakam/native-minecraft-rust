from pathlib import Path
import sys


def read(path):
    rows={}
    for line in Path(path).read_text(encoding='utf-8').splitlines():
        if not line.startswith('celestial '):
            continue
        prefix,time,partial,bits,moon=line.split()
        key=(int(time),float(partial))
        if key in rows:
            raise ValueError('Duplicate celestial observation')
        rows[key]=(int(bits),int(moon))
    if rows.keys() != {(t,p) for t in range(24000) for p in [0,.25,.5,.75]}:
        raise ValueError('Incomplete celestial observations')
    return rows


if __name__=='__main__':
    actual,reference=map(read,sys.argv[1:])
    diff=[key for key in reference if reference[key]!=actual[key]]
    if diff:
        print([(key,actual[key],reference[key]) for key in diff[:5]])
        raise SystemExit(f'Celestial differences: {len(diff)}')
    print('Celestial angles: 96000 observations, 0 bit differences')
