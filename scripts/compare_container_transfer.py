"""Exact comparison of authored container transfer scenarios and full slot state."""
from pathlib import Path
import sys


def read(path):
    rows = {}
    for line in Path(path).read_text(encoding='utf-8').splitlines():
        if not line.startswith('transfer '):
            continue
        fields = line.split()
        key = tuple(fields[1:6])
        if key in rows:
            raise ValueError(f'Duplicate transfer observation: {key}')
        rows[key] = tuple(fields[6:])
    expected = {(kind,str(slot),str(item),str(count),str(pattern))
                for kind,size in [('chest',27),('hopper',5),('dispenser',9),('dropper',9),('beacon',1)]
                for slot in range(size+36) for item in [1,339,35,264,265]
                for count in [1,32] for pattern in range(3)}
    if rows.keys() != expected:
        raise ValueError('Missing or unexpected transfer observations')
    for key, values in rows.items():
        size = {'chest':27,'hopper':5,'dispenser':9,'dropper':9,'beacon':1}[key[0]]
        if len(values) != size+37:
            raise ValueError(f'Missing slot state: {key}')
        for value in values:
            if value != '-':
                parts = value.split(':')
                if len(parts) != 3 or any(not part.isdigit() for part in parts):
                    raise ValueError(f'Malformed stack: {value}')
    return rows


if __name__ == '__main__':
    actual, reference = map(read, sys.argv[1:])
    differences = [key for key in reference if actual[key] != reference[key]]
    if differences:
        for key in differences[:5]:
            print(key, actual[key], reference[key])
        raise SystemExit(f'Transfer differences: {len(differences)}')
    print(f'Container transfers: {len(actual)} scenarios, 0 differences')
