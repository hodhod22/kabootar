#!/usr/bin/env python3
"""Split one emit shard — ONLY if/else chains at depth 0 (safe). Disabled by default."""
from __future__ import annotations

import sys

print(
    "ERROR: _split_emit_shards_batch.py is disabled — it corrupts if/else blocks.\n"
    "Use scripts/_split_emit_call_helpers.py for call shards, or manual extraction.\n"
    "See emit_stmt_let_ctor.kab for the let-stmt pattern.",
    file=sys.stderr,
)
sys.exit(1)
