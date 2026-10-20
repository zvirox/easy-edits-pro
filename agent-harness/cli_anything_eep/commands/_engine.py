"""Shared execution helpers for engine-backed CLI verbs."""

from __future__ import annotations

import json
from typing import Any, Optional

import click

from cli_anything_eep.core import backend, errors
from cli_anything_eep.output import format_output, format_success


def run_engine(
    ctx: click.Context,
    command_id: str,
    params: Optional[dict[str, Any]] = None,
    *,
    success: Optional[str] = None,
):
    """Execute one registry command and print its result or structured error."""
    try:
        result = backend.get_saturn_cli().execute(command_id, params or {})
        if success:
            payload = result if isinstance(result, dict) else {"result": result}
            format_success(success, payload, ctx.obj['output_format'])
        else:
            format_output(result, ctx.obj['output_format'])
    except errors.EepError as e:
        format_output(e.to_dict(), ctx.obj['output_format'])
        ctx.exit(1)


def parse_json_option(value: Optional[str], label: str) -> Any:
    """Parse a JSON option, raising a structured CLI error when it is invalid."""
    if value is None:
        return None
    try:
        return json.loads(value)
    except json.JSONDecodeError as error:
        raise errors.EepError(
            errors.ErrorCode.INVALID_ARGUMENT,
            f"{label} must be valid JSON: {error}",
            details={"value": value},
        ) from error


def clip_refs(value: str) -> list[dict[str, int]]:
    """Parse `track:index,track:index` into clip references."""
    references = []
    for item in value.split(','):
        item = item.strip()
        if not item:
            continue
        try:
            track, index = item.split(':', 1)
            references.append({"track_id": int(track), "clip_index": int(index)})
        except ValueError as error:
            raise errors.EepError(
                errors.ErrorCode.INVALID_ARGUMENT,
                f"Clip reference must be track:index, got {item!r}",
            ) from error
    if not references:
        raise errors.EepError(errors.ErrorCode.INVALID_ARGUMENT, "At least one clip reference is required")
    return references


def tick_ranges(value: str) -> list[list[int]]:
    """Parse `start:end,start:end` into tick ranges."""
    ranges = []
    for item in value.split(','):
        item = item.strip()
        if not item:
            continue
        try:
            start, end = item.split(':', 1)
            ranges.append([int(start), int(end)])
        except ValueError as error:
            raise errors.EepError(
                errors.ErrorCode.INVALID_ARGUMENT,
                f"Range must be start:end, got {item!r}",
            ) from error
    if not ranges:
        raise errors.EepError(errors.ErrorCode.INVALID_ARGUMENT, "At least one range is required")
    return ranges


def comma_list(value: Optional[str]) -> Optional[list[str]]:
    if value is None:
        return None
    return [item.strip() for item in value.split(',') if item.strip()]
