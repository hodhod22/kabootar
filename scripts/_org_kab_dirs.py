# One-shot: nest lib/kab/*.kab into prefix subdirs; rewrite imports/paths.
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
KAB = ROOT / "lib" / "kab"
SKIP_DIRS = {
    ".git",
    "node_modules",
    "target",
    "target-sh6cv",
    "target-sh6",
    "target-sh",
    "target-sh2",
    "target-sh8rel",
    "target-shfix",
    "target-h6e",
    "target-h6e2",
    "target-h6e3",
    "target-h6e4",
    "target-h6e5",
    "target-local2",
    "target-local3",
    "target-p6b",
    "target-p6b2",
    "target-p6b3",
    "target-p6b4",
    "target-p6b5",
    "target-p6b6",
    "target-p6b7",
    "target-p6b8",
    "target-p6b8-rel",
    "target-p6b9",
    "target-p6b9-rel",
    "target-p6b-rel",
    "target-emit-densify",
}
TEXT_SUFFIX = {".kab", ".rs", ".md", ".txt", ".toml", ".json"}
GROUPS = [
    ("ui_", "ui"),
    ("jit_", "jit"),
    ("aot_", "aot"),
    ("noll_", "noll"),
    ("std_", "std"),
    ("http_", "http"),
    ("cli_", "cli"),
    ("load_", "load"),
    ("gc_", "gc"),
    ("sql_", "sql"),
    ("os_", "os"),
    ("sci_", "sci"),
    ("crypto_", "crypto"),
    ("io_", "io"),
]


def dest_for(stem: str):
    for prefix, sub in GROUPS:
        if stem.startswith(prefix):
            return sub
    return None


def main():
    files = sorted(KAB.glob("*.kab"))
    moves = []
    for f in files:
        sub = dest_for(f.stem)
        if sub is None:
            continue
        d = KAB / sub
        d.mkdir(exist_ok=True)
        dest = d / f.name
        if dest.exists():
            print("exists", dest, file=sys.stderr)
            sys.exit(1)
        moves.append((f, dest, f.stem, sub))
    print("moves", len(moves))
    for src, dest, _stem, _sub in moves:
        dest.parent.mkdir(exist_ok=True)
        r = subprocess.run(
            ["git", "ls-files", "--error-unmatch", str(src)],
            cwd=ROOT,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        if r.returncode == 0:
            subprocess.check_call(["git", "mv", str(src), str(dest)], cwd=ROOT)
        else:
            src.replace(dest)
    repls = []
    for _src, _dest, stem, sub in moves:
        old_imp = f"kab/{stem}"
        new_imp = f"kab/{sub}/{stem}"
        old_path = f"lib/kab/{stem}.kab"
        new_path = f"lib/kab/{sub}/{stem}.kab"
        repls.append((old_imp, new_imp, old_path, new_path, len(stem)))
    repls.sort(key=lambda t: -t[4])
    nfiles = 0
    nsubs = 0
    for path in ROOT.rglob("*"):
        if not path.is_file():
            continue
        if path.suffix.lower() not in TEXT_SUFFIX:
            continue
        parts = set(path.parts)
        if parts & SKIP_DIRS:
            continue
        if "target-" in str(path):
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except (UnicodeDecodeError, OSError):
            continue
        orig = text
        for old_imp, new_imp, old_path, new_path, _n in repls:
            if old_imp in text:
                text = text.replace(old_imp, new_imp)
            if old_path in text:
                text = text.replace(old_path, new_path)
        if text != orig:
            path.write_text(text, encoding="utf-8", newline="\n")
            nfiles += 1
            nsubs += 1
    print("rewritten files", nfiles)
    top = list(KAB.glob("*.kab"))
    print("top-level kab", len(top))


if __name__ == "__main__":
    main()
