"""Translate numeric local mining observations into independent Rust lookup rules."""
from collections import defaultdict
from pathlib import Path
import sys

def main():
    blocks, tools, names = {}, defaultdict(list), {}
    for line in Path(sys.argv[1]).read_text(encoding='utf-8-sig').splitlines():
        p = line.split()
        if p and p[0] == 'mining':
            blocks[int(p[1])] = (float(p[2]), p[3] == 'true')
            names[int(p[1])] = p[4]
        elif p and p[0] == 'tool':
            tools[int(p[2])].append((int(p[1]), float(p[3]), p[4] == 'true'))
    if len(blocks) != 198 or any(len(values) != 198 for values in tools.values()):
        sys.exit('Incomplete local mining observations')
    lines = ['//! Numeric block/tool behavior observed with the authored local MCP probe.',
             '//! Regenerate with scripts/generate_mining_properties.py; contains no game assets.',
             'pub fn block_properties(id: u16) -> Option<(f32, bool)> {', '    match id {']
    groups = defaultdict(list)
    for id, value in blocks.items(): groups[value].append(id)
    for (hardness, hand), ids in sorted(groups.items()):
        lines.append(f'        {" | ".join(map(str,ids))} => Some(({hardness!r}, {str(hand).lower()})),')
    lines.extend(['        _ => None,', '    }', '}',
                  'pub fn tool_properties(block: u16, item: i16, hand: bool) -> (f32, bool) {',
                  '    match (item, block) {'])
    for tool, values in sorted(tools.items()):
        groups = defaultdict(list)
        for block, speed, harvest in values:
            if speed != 1.0 or harvest != blocks[block][1]: groups[(speed,harvest)].append(block)
        for (speed, harvest), ids in sorted(groups.items()):
            lines.append(f'        ({tool}, {" | ".join(map(str,ids))}) => ({speed!r}, {str(harvest).lower()}),')
    lines.extend(['        _ => (1.0, hand),', '    }', '}', 'pub fn block_name(id: u16) -> Option<&\'static str> {', '    match id {'])
    for id, name in sorted(names.items()): lines.append(f'        {id} => Some("{name}"),')
    lines.extend(['        _ => None,', '    }', '}'])
    Path(sys.argv[2]).write_text('\n'.join(lines)+'\n', encoding='utf-8')

if __name__ == '__main__': main()
