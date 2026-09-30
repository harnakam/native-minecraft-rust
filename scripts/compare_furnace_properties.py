"""Compare complete numeric fuel and smelting observations, rejecting partial data."""
import sys
from pathlib import Path


def read(path):
    rows = {}
    for line in Path(path).read_text(encoding='utf-8').splitlines():
        fields = line.split()
        if not fields or fields[0] not in ('fuel','smelt'):
            continue
        if len(fields) != (3 if fields[0]=='fuel' else 6):
            raise ValueError('Malformed furnace observation')
        values = tuple(map(int,fields[1:]))
        key = (fields[0], *values[:1 if fields[0]=='fuel' else 2])
        if key in rows:
            raise ValueError('Duplicate furnace observation')
        rows[key] = values
    if sum(k[0]=='fuel' for k in rows)!=337 or sum(k[0]=='smelt' for k in rows)!=26:
        raise ValueError('Incomplete furnace registry')
    return rows


if __name__ == '__main__':
    actual, reference = map(read, sys.argv[1:])
    if actual != reference:
        raise SystemExit('Furnace registry differs')
    print('Furnace properties: 337 fuels and 26 recipes, 0 differences')
