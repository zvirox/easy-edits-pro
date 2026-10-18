use super::*;

impl EditorEngine {
    pub(super) fn dispatch_nodegraph(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        match id {
            "nodegraph.add_node" => Some((|| {
                let params: AddNodeParams = parse_params(params)?;
                self.checkpoint();
                let node_id = self.document.project.next_id;
                self.document.project.next_id = node_id.saturating_add(1);
                let graph = get_or_create_node_graph(
                    &mut self.document,
                    params.track_id,
                    params.clip_index,
                )?;
                graph.nodes.push(CompositingNode {
                    id: node_id,
                    kind: params.kind,
                    title: params.title,
                    pos_x: params.pos_x,
                    pos_y: params.pos_y,
                    params: params.params,
                    enabled: true,
                });
                Ok(json!({"added": true, "node_id": node_id}))
            })()),
            "nodegraph.remove_node" => Some((|| {
                let params: RemoveNodeParams = parse_params(params)?;
                self.checkpoint();
                let graph = get_or_create_node_graph(
                    &mut self.document,
                    params.track_id,
                    params.clip_index,
                )?;
                let initial_len = graph.nodes.len();
                graph.nodes.retain(|n| n.id != params.node_id);
                let removed = graph.nodes.len() < initial_len;
                if removed {
                    graph
                        .connections
                        .retain(|c| c.from_node != params.node_id && c.to_node != params.node_id);
                }
                Ok(json!({"removed": removed}))
            })()),
            "nodegraph.connect" => Some((|| {
                let params: ConnectNodesParams = parse_params(params)?;
                self.checkpoint();
                let graph = get_or_create_node_graph(
                    &mut self.document,
                    params.track_id,
                    params.clip_index,
                )?;
                if !graph.connections.iter().any(|c| {
                    c.from_node == params.from_node
                        && c.from_socket == params.from_socket
                        && c.to_node == params.to_node
                        && c.to_socket == params.to_socket
                }) {
                    graph.connections.push(NodeConnection {
                        from_node: params.from_node,
                        from_socket: params.from_socket,
                        to_node: params.to_node,
                        to_socket: params.to_socket,
                    });
                }
                Ok(json!({"connected": true}))
            })()),
            "nodegraph.disconnect" => Some((|| {
                let params: DisconnectNodesParams = parse_params(params)?;
                self.checkpoint();
                let graph = get_or_create_node_graph(
                    &mut self.document,
                    params.track_id,
                    params.clip_index,
                )?;
                let initial_len = graph.connections.len();
                graph.connections.retain(|c| {
                    !(c.from_node == params.from_node
                        && c.from_socket == params.from_socket
                        && c.to_node == params.to_node
                        && c.to_socket == params.to_socket)
                });
                Ok(json!({"disconnected": graph.connections.len() < initial_len}))
            })()),
            "nodegraph.update_node" => Some((|| {
                let params: UpdateNodeParams = parse_params(params)?;
                self.checkpoint();
                let graph = get_or_create_node_graph(
                    &mut self.document,
                    params.track_id,
                    params.clip_index,
                )?;
                let node = graph
                    .nodes
                    .iter_mut()
                    .find(|n| n.id == params.node_id)
                    .ok_or_else(|| {
                        DispatchError::Operation(format!("Node {} not found", params.node_id))
                    })?;
                if let Some(x) = params.pos_x {
                    node.pos_x = x;
                }
                if let Some(y) = params.pos_y {
                    node.pos_y = y;
                }
                if let Some(e) = params.enabled {
                    node.enabled = e;
                }
                if let Some(p) = params.params {
                    node.params = p;
                }
                Ok(json!({"updated": true, "node_id": params.node_id}))
            })()),
            "nodegraph.add_mask" => Some((|| {
                let params: AddMaskParams = parse_params(params)?;
                self.checkpoint();
                let mask_id = if params.mask.id == 0 {
                    let id = self.document.project.next_id;
                    self.document.project.next_id = id.saturating_add(1);
                    id
                } else {
                    params.mask.id
                };
                let mut mask = params.mask;
                mask.id = mask_id;
                let graph = get_or_create_node_graph(
                    &mut self.document,
                    params.track_id,
                    params.clip_index,
                )?;
                graph.masks.push(mask);
                Ok(json!({"added": true, "mask_id": mask_id}))
            })()),
            _ => None,
        }
    }
}
