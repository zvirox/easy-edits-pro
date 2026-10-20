"""Node graph commands for Easy Edits Pro."""

import click

from cli_anything_eep.commands import _engine

NODE_KINDS = ['merge', 'transform', 'keyer', 'mask', 'particles', 'media_source', 'output_sink']


def _context_params(track, clip_index):
    params = {}
    if track is not None:
        params["track_id"] = track
    if clip_index is not None:
        params["clip_index"] = clip_index
    return params


@click.group(name='nodegraph')
def nodegraph_group():
    """Compositing node graph commands."""
    pass


@nodegraph_group.command('add-node')
@click.option('--kind', required=True, type=click.Choice(NODE_KINDS), help='Node kind')
@click.option('--title', default='', help='Node title')
@click.option('--pos-x', type=float, default=0.0, help='Canvas X position')
@click.option('--pos-y', type=float, default=0.0, help='Canvas Y position')
@click.option('--params', help='Node parameters as JSON')
@click.option('--track', type=int, help='Track id the graph belongs to')
@click.option('--clip-index', type=int, help='Clip index the graph belongs to')
@click.pass_context
def nodegraph_add_node(ctx: click.Context, kind: str, title: str, pos_x: float, pos_y: float, params, track, clip_index):
    """Add a compositing node."""
    payload = _context_params(track, clip_index)
    payload.update({"kind": kind, "title": title, "pos_x": pos_x, "pos_y": pos_y})
    if params is not None:
        payload["params"] = _engine.parse_json_option(params, '--params')
    _engine.run_engine(ctx, "nodegraph.add_node", payload, success="Node added")


@nodegraph_group.command('connect')
@click.option('--from-node', type=int, required=True, help='Source node id')
@click.option('--from-socket', required=True, help='Source socket')
@click.option('--to-node', type=int, required=True, help='Target node id')
@click.option('--to-socket', required=True, help='Target socket')
@click.option('--track', type=int, help='Track id the graph belongs to')
@click.option('--clip-index', type=int, help='Clip index the graph belongs to')
@click.pass_context
def nodegraph_connect(ctx: click.Context, from_node: int, from_socket: str, to_node: int, to_socket: str, track, clip_index):
    """Connect two node sockets."""
    payload = _context_params(track, clip_index)
    payload.update({"from_node": from_node, "from_socket": from_socket, "to_node": to_node, "to_socket": to_socket})
    _engine.run_engine(ctx, "nodegraph.connect", payload, success="Nodes connected")


@nodegraph_group.command('disconnect')
@click.option('--from-node', type=int, required=True, help='Source node id')
@click.option('--from-socket', required=True, help='Source socket')
@click.option('--to-node', type=int, required=True, help='Target node id')
@click.option('--to-socket', required=True, help='Target socket')
@click.option('--track', type=int, help='Track id the graph belongs to')
@click.option('--clip-index', type=int, help='Clip index the graph belongs to')
@click.pass_context
def nodegraph_disconnect(ctx: click.Context, from_node: int, from_socket: str, to_node: int, to_socket: str, track, clip_index):
    """Disconnect two node sockets."""
    payload = _context_params(track, clip_index)
    payload.update({"from_node": from_node, "from_socket": from_socket, "to_node": to_node, "to_socket": to_socket})
    _engine.run_engine(ctx, "nodegraph.disconnect", payload, success="Nodes disconnected")


@nodegraph_group.command('remove-node')
@click.option('--node-id', type=int, required=True, help='Node id')
@click.option('--track', type=int, help='Track id the graph belongs to')
@click.option('--clip-index', type=int, help='Clip index the graph belongs to')
@click.pass_context
def nodegraph_remove_node(ctx: click.Context, node_id: int, track, clip_index):
    """Remove a node."""
    payload = _context_params(track, clip_index)
    payload["node_id"] = node_id
    _engine.run_engine(ctx, "nodegraph.remove_node", payload, success="Node removed")


@nodegraph_group.command('update-node')
@click.option('--node-id', type=int, required=True, help='Node id')
@click.option('--pos-x', type=float, help='New canvas X position')
@click.option('--pos-y', type=float, help='New canvas Y position')
@click.option('--enabled/--disabled', default=None, help='Enable or bypass the node')
@click.option('--params', help='Merged node parameters as JSON')
@click.option('--track', type=int, help='Track id the graph belongs to')
@click.option('--clip-index', type=int, help='Clip index the graph belongs to')
@click.pass_context
def nodegraph_update_node(ctx: click.Context, node_id: int, pos_x, pos_y, enabled, params, track, clip_index):
    """Update a node's position, state, or parameters."""
    payload = _context_params(track, clip_index)
    payload["node_id"] = node_id
    for key, value in (("pos_x", pos_x), ("pos_y", pos_y), ("enabled", enabled)):
        if value is not None:
            payload[key] = value
    if params is not None:
        payload["params"] = _engine.parse_json_option(params, '--params')
    _engine.run_engine(ctx, "nodegraph.update_node", payload, success="Node updated")


@nodegraph_group.command('add-mask')
@click.option('--mask', required=True, help='RotoSplineMask JSON')
@click.option('--track', type=int, help='Track id the graph belongs to')
@click.option('--clip-index', type=int, help='Clip index the graph belongs to')
@click.pass_context
def nodegraph_add_mask(ctx: click.Context, mask: str, track, clip_index):
    """Add a roto spline mask to the graph."""
    payload = _context_params(track, clip_index)
    payload["mask"] = _engine.parse_json_option(mask, '--mask')
    _engine.run_engine(ctx, "nodegraph.add_mask", payload, success="Mask added")
