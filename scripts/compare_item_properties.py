"""Compare complete numeric item registry observations from MCP919 and Rust."""
import sys
from pathlib import Path


def read(path):
    rows = {}
    for line in Path(path).read_text(encoding='utf-8').splitlines():
        if not line.startswith('item_property '):
            continue
        fields = line.split()
        if len(fields) != 5 or fields[3] not in ('true', 'false'):
            raise ValueError(f'Malformed registry observation: {line}')
        item, limit, damage = int(fields[1]), int(fields[2]), int(fields[4])
        if item in rows or not 0 <= item < 32768 or not 1 <= limit <= 64 or damage < 0:
            raise ValueError(f'Invalid or duplicate registry observation: {line}')
        rows[item] = (limit, fields[3] == 'true', damage)
    if len(rows) != 337:
        raise ValueError(f'Expected all 337 registered 1.8.9 items, found {len(rows)}')
    return rows


if __name__ == '__main__':
    actual, reference = map(read, sys.argv[1:])
    differences = [item for item in actual.keys() | reference.keys()
                   if actual.get(item) != reference.get(item)]
    if differences:
        raise SystemExit(f'Item property differences: {differences}')
    print(f'Item registry: {len(actual)} items, 0 differences')
