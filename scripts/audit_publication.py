"""Fail closed on files inappropriate for the public source repository.

Run after staging the explicit source allowlist, before committing or pushing.
No Minecraft installation, third-party package, or network access is required.
"""
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
ALLOWED_ROOTS = {"crates", "docs", "scripts", ".github"}
ALLOWED_FILES = {"Cargo.toml", "Cargo.lock", "README.md", ".gitignore", ".gitattributes", "Run RustMinecraft.bat"}
EXTENSIONS = {".rs", ".toml", ".lock", ".md", ".bat", ".py", ".yml", ".yaml", ".java"}
SECRET = re.compile(r"(?:gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{50,}|AKIA[A-Z0-9]{16}|-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----)")


def inspect(name: str) -> list[str]:
    path = Path(name)
    problems = []
    if name not in ALLOWED_FILES and (path.parts[0] not in ALLOWED_ROOTS or path.suffix not in EXTENSIONS):
        problems.append("outside publication allowlist")
    if path.suffix == ".java" and not name.startswith("crates/rmc-java/templates/Rmc"):
        problems.append("Java reference source is not distributable")
    source = ROOT / path
    if source.is_symlink():
        return problems + ["symlinks are not allowed"]
    try:
        data = source.read_bytes()
        content = data.decode("utf-8")
        if b"\0" in data:
            problems.append("binary content")
        if SECRET.search(content):
            problems.append("credential material")
    except (OSError, UnicodeError):
        problems.append("missing or non-text file")
    return problems


def main() -> int:
    names = subprocess.check_output(["git", "ls-files", "-z"], cwd=ROOT).decode("utf-8").split("\0")
    files = [name for name in names if name]
    failures = [(name, inspect(name)) for name in files]
    failures = [(name, reasons) for name, reasons in failures if reasons]
    for name, reasons in failures:
        print(f"REJECTED {name}: {', '.join(reasons)}")
    if failures:
        return 1
    print(f"Publication audit passed: {len(files)} tracked text files; no game assets, archives, or detected credentials.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
