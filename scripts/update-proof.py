#!/usr/bin/env python3
"""Copy the body of docs/testnet-proof.md into the README proof section.

Usage: scripts/update-proof.py [proof.md] [README.md]

Replaces everything between <!-- proof:start --> and <!-- proof:end -->.
Safe to run repeatedly.
"""
import re
import sys

proof_path = sys.argv[1] if len(sys.argv) > 1 else "docs/testnet-proof.md"
readme_path = sys.argv[2] if len(sys.argv) > 2 else "README.md"

proof = open(proof_path, encoding="utf-8").read().strip()
# Drop the proof file's own H1; the README already has a section heading.
proof = re.sub(r"\A# .*\n+", "", proof)

readme = open(readme_path, encoding="utf-8").read()
pattern = re.compile(r"(<!-- proof:start -->\n).*?(\n<!-- proof:end -->)", re.DOTALL)
if not pattern.search(readme):
    sys.exit(f"markers not found in {readme_path}")

# Use a function so backslashes in the proof text are never treated as escapes.
updated = pattern.sub(lambda m: m.group(1) + proof + m.group(2), readme, count=1)
open(readme_path, "w", encoding="utf-8").write(updated)
print(f"updated {readme_path} from {proof_path}")
