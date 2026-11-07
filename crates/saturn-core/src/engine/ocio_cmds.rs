// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Scene-referred colour-management commands (Packet 3.1):
//! `ocio.set_pipeline` and `ocio.set_media_idt`. Validation is honest:
//! unknown space names fail deserialisation into the core enums and surface
//! as `InvalidParameters`, never a silent default.

use super::*;

impl EditorEngine {
    pub(super) fn dispatch_ocio(
        &mut self,
        id: &str,
        params: Value,
    ) -> Option<Result<Value, DispatchError>> {
        match id {
            "ocio.set_pipeline" => Some((|| {
                let params: SetPipelineParams = parse_params(params)?;
                self.checkpoint();
                let cm = &mut self.document.project.color_management;
                cm.working_space = params.working_space;
                cm.display_odt = params.display_odt;
                cm.default_idt = params.default_idt;
                Ok(serde_json::json!({
                    "workingSpace": cm.working_space.as_str(),
                    "displayOdt": cm.display_odt.as_str(),
                    "defaultIdt": cm.default_idt.map(|idt| idt.as_str()),
                }))
            })()),
            "ocio.set_media_idt" => Some((|| {
                let params: SetMediaIdtParams = parse_params(params)?;
                let item = self
                    .document
                    .project
                    .media
                    .iter_mut()
                    .find(|asset| asset.id == params.media_id)
                    .ok_or_else(|| {
                        DispatchError::InvalidParameters(format!(
                            "Media asset {} does not exist",
                            params.media_id
                        ))
                    })?;
                item.idt = Some(params.idt);
                Ok(serde_json::json!({
                    "mediaId": item.id,
                    "idt": params.idt.as_str(),
                }))
            })()),
            _ => None,
        }
    }
}
