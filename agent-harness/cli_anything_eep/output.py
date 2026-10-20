"""
Output formatting for Easy Edits Pro CLI.
Supports JSON (agents), text (humans), table (data).
"""

import json
import click
from typing import Any, Dict, List, Optional


def report_unsupported(ctx, command: str, reason: str) -> None:
    """Emit an honest structured error for features without an engine implementation."""
    from cli_anything_eep.core.errors import EepError, ErrorCode
    format_output(
        EepError(ErrorCode.COMMAND_NOT_FOUND, f"{command} is unavailable: {reason}", details={"command": command}, recoverable=False).to_dict(),
        ctx.obj['output_format'],
    )
    ctx.exit(1)
from tabulate import tabulate


def format_output(data: Any, format_type: str = 'text', **kwargs) -> None:
    """Format and output data according to format type."""
    if format_type == 'json':
        click.echo(json.dumps(data, indent=2, default=str))
    elif format_type == 'table':
        _output_table(data, **kwargs)
    else:
        _output_text(data, **kwargs)


def _output_text(data: Any, **kwargs) -> None:
    """Human-readable text output."""
    if data is None:
        return
    
    if isinstance(data, dict):
        if 'error' in data:
            _output_error(data)
        elif 'message' in data:
            click.echo(data['message'])
        else:
            _output_dict(data)
    elif isinstance(data, list):
        for item in data:
            if isinstance(item, dict):
                _output_dict(item)
                click.echo("---")
            else:
                click.echo(f"  {item}")
    else:
        click.echo(str(data))


def _output_dict(data: Dict[str, Any], indent: int = 0) -> None:
    """Output dictionary in readable format."""
    prefix = "  " * indent
    for key, value in data.items():
        if isinstance(value, dict):
            click.echo(f"{prefix}{key}:")
            _output_dict(value, indent + 1)
        elif isinstance(value, list):
            click.echo(f"{prefix}{key}:")
            for i, item in enumerate(value):
                if isinstance(item, dict):
                    click.echo(f"{prefix}  [{i}]")
                    _output_dict(item, indent + 2)
                else:
                    click.echo(f"{prefix}  - {item}")
        else:
            click.echo(f"{prefix}{key}: {value}")


def _output_error(data: Dict[str, Any]) -> None:
    """Output error in readable format."""
    click.echo(f"Error: {data.get('message', 'Unknown error')}", err=True)
    if data.get('details'):
        click.echo("Details:", err=True)
        _output_dict(data['details'], 1)


def _output_table(data: Any, headers: Optional[List[str]] = None, **kwargs) -> None:
    """Table output for structured data."""
    if not data:
        click.echo("(empty)")
        return
    
    if isinstance(data, dict):
        # Single row
        rows = [data]
    elif isinstance(data, list):
        rows = data
    else:
        click.echo(str(data))
        return
    
    if not rows:
        click.echo("(empty)")
        return
    
    if headers is None:
        headers = list(rows[0].keys()) if rows else []
    
    table_data = []
    for row in rows:
        if isinstance(row, dict):
            table_data.append([row.get(h, '') for h in headers])
        else:
            table_data.append([str(row)])
    
    click.echo(tabulate(table_data, headers=headers, tablefmt='simple'))


def format_success(message: str, data: Optional[Dict] = None, format_type: str = 'text') -> None:
    """Format success response."""
    if format_type == 'json':
        result = {"success": True, "message": message}
        if data:
            result["data"] = data
        click.echo(json.dumps(result, indent=2, default=str))
    else:
        click.echo(f"✓ {message}")
        if data:
            _output_dict(data)


def format_progress(current: int, total: int, prefix: str = "", format_type: str = 'text') -> None:
    """Format progress indicator."""
    if format_type == 'json':
        click.echo(json.dumps({"progress": current, "total": total, "prefix": prefix}))
    else:
        pct = (current / total * 100) if total > 0 else 0
        bar_len = 30
        filled = int(bar_len * current / total) if total > 0 else 0
        bar = "█" * filled + "░" * (bar_len - filled)
        click.echo(f"\r{prefix} [{bar}] {pct:.1f}% ({current}/{total})", nl=False)
        if current >= total:
            click.echo()


def format_command_help(commands: List[Dict], format_type: str = 'text') -> None:
    """Format command listing."""
    if format_type == 'json':
        click.echo(json.dumps(commands, indent=2))
    else:
        for cmd in commands:
            name = cmd.get('name', '')
            desc = cmd.get('description', '')
            params = cmd.get('params', [])
            click.echo(f"  {name}")
            if desc:
                click.echo(f"    {desc}")
            if params:
                click.echo(f"    Params: {', '.join(p.get('name', '') for p in params)}")
            click.echo()
