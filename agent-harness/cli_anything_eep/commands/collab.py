"""Collaboration lock and change-log commands."""

import click

from cli_anything_eep.commands import _engine

TARGET_KINDS = ['bin', 'sequence', 'track', 'clip']


@click.group(name='collab')
def collab_group():
    """Collaboration locks and change records."""
    pass


@collab_group.command('acquire-lock')
@click.option('--user-id', required=True, help='Stable user id')
@click.option('--user-name', required=True, help='Display name')
@click.option('--user-color', help='Cursor color')
@click.option('--target-kind', required=True, type=click.Choice(TARGET_KINDS), help='Locked object kind')
@click.option('--target-id', type=int, required=True, help='Locked object id')
@click.pass_context
def collab_acquire(ctx: click.Context, user_id: str, user_name: str, user_color, target_kind: str, target_id: int):
    """Acquire an edit lock."""
    params = {
        "user_id": user_id,
        "user_name": user_name,
        "target_kind": target_kind,
        "target_id": target_id,
    }
    if user_color is not None:
        params["user_color"] = user_color
    _engine.run_engine(ctx, "collab.acquire_lock", params, success="Lock acquired")


@collab_group.command('release-lock')
@click.option('--user-id', required=True, help='Stable user id')
@click.option('--target-kind', required=True, type=click.Choice(TARGET_KINDS), help='Locked object kind')
@click.option('--target-id', type=int, required=True, help='Locked object id')
@click.pass_context
def collab_release(ctx: click.Context, user_id: str, target_kind: str, target_id: int):
    """Release an edit lock."""
    _engine.run_engine(
        ctx,
        "collab.release_lock",
        {"user_id": user_id, "target_kind": target_kind, "target_id": target_id},
        success="Lock released",
    )


@collab_group.command('record-change')
@click.option('--user-id', required=True, help='Stable user id')
@click.option('--user-name', required=True, help='Display name')
@click.option('--action', required=True, help='Action label')
@click.option('--summary', required=True, help='Human-readable summary')
@click.pass_context
def collab_record_change(ctx: click.Context, user_id: str, user_name: str, action: str, summary: str):
    """Append a collaboration change record."""
    _engine.run_engine(
        ctx,
        "collab.record_change",
        {"user_id": user_id, "user_name": user_name, "action": action, "summary": summary},
        success="Change recorded",
    )
