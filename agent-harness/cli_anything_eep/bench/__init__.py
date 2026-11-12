# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
"""Scripted EEP-Bench benchmark definitions and deterministic runner."""

from __future__ import annotations

from .runner import run_bench_suite, run_bench_task
from .tasks import BENCH_TASKS, list_bench_tasks

__all__ = [
    "BENCH_TASKS",
    "list_bench_tasks",
    "run_bench_suite",
    "run_bench_task",
]
