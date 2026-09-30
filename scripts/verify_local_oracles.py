"""Run authored behavioral probes against a local MCP919 build; publish no game files."""
import argparse
import os
from pathlib import Path
import subprocess
import sys


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--mcp-root', type=Path, default=Path('MCP-919'))
    parser.add_argument('--java-home', type=Path, required=True)
    parser.add_argument('--cargo', default='cargo')
    parser.add_argument('--only', choices=['collision','neighbors','travel','mining','minestate','selection','explosion','item_properties','container_transfer','furnace_properties'])
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    mcp = args.mcp_root.resolve()
    java, javac = [args.java_home.resolve() / 'bin' / name for name in ('java', 'javac')]
    if os.name == 'nt':
        java, javac = [path.with_suffix('.exe') for path in (java, javac)]
    classes = mcp / 'bin/minecraft'
    jar = mcp / 'jars/versions/1.8.9/1.8.9.jar'
    libraries = sorted((mcp / 'jars/libraries').rglob('*.jar'))
    for path in (java, javac, classes, jar):
        if not path.exists():
            parser.error(f'Required local file or directory missing: {path}')
    if not libraries:
        parser.error('MCP libraries missing')
    # Relative paths also avoid Windows Java 8 Unicode classpath canonicalization issues.
    classpath = os.pathsep.join(os.path.relpath(path, root) for path in [classes, jar, *libraries])
    templates = root / 'crates/rmc-java/templates'
    subprocess.run([str(javac), '-encoding', 'UTF-8', '-source', '8', '-target', '8',
                    '-cp', classpath, '-d', os.path.relpath(classes, root),
                    str(templates / 'RmcCollisionOracle.java'),
                    str(templates / 'RmcMovementOracle.java'),
                    str(templates / 'RmcMiningOracle.java'),
                    str(templates / 'RmcItemPropertiesOracle.java'),
                    str(templates / 'RmcFurnacePropertiesOracle.java'),
                    str(templates / 'RmcContainerTransferOracle.java'),
                    str(templates / 'RmcMiningStateOracle.java'),
                    str(templates / 'RmcExplosionOracle.java'),
                    str(templates / 'RmcNeighborOracle.java'),
                    str(templates / 'RmcSelectionOracle.java')], cwd=root, check=True)
    output = root / 'tmp/local-oracles'
    output.mkdir(parents=True, exist_ok=True)
    for name, class_name, package, example in (
        ('furnace_properties', 'RmcFurnacePropertiesOracle', 'rmc-game', 'furnace_properties_dump'),
        ('container_transfer', 'RmcContainerTransferOracle', 'rmc-game', 'container_transfer_dump'),
        ('item_properties', 'RmcItemPropertiesOracle', 'rmc-game', 'item_properties_dump'),
        ('explosion', 'RmcExplosionOracle', 'rmc-net', 'explosion_dump'),
        ('neighbors', 'RmcNeighborOracle', 'rmc-world', 'neighbor_dump'),
        ('collision', 'RmcCollisionOracle', 'rmc-world', 'collision_dump'),
        ('travel', 'RmcMovementOracle', 'rmc-game', 'travel_dump'),
        ('mining', 'RmcMiningOracle', 'rmc-game', 'mining_dump'),
        ('minestate', 'RmcMiningStateOracle', 'rmc-game', 'minestate_dump'),
        ('selection', 'RmcSelectionOracle', 'rmc-world', 'selection_dump'),
    ):
        if args.only and args.only != name:
            continue
        reference, actual = output / f'{name}-java.txt', output / f'{name}-rust.txt'
        for command, destination in (
            ([str(java), '-cp', classpath, f'net.minecraft.client.rmc.{class_name}'], reference),
            ([args.cargo, 'run', '--locked', '--offline', '-p', package, '--example', example], actual),
        ):
            with destination.open('w', encoding='utf-8') as stream:
                subprocess.run(command, cwd=root, stdout=stream, check=True)
        subprocess.run([sys.executable, str(root / f"scripts/compare_{'collision' if name == 'neighbors' else name}.py"),
                        str(actual), str(reference)], cwd=root, check=True)


if __name__ == '__main__':
    main()
