// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
export { GraphicsPanel } from "./GraphicsPanel.tsx";
export { CanvasGraphicsLayer } from "./CanvasGraphicsLayer.tsx";
export type { GraphicsEditing, GraphicsCanvasTool } from "./CanvasGraphicsLayer.tsx";
export {
  activeGraphics,
  fillToCss,
  overlayBox,
  shadowToCss,
  shapeBackgroundCss,
  strokeToCss,
  textOverlayStyle,
} from "./graphicsOverlay.ts";
export type { GraphicOverlayBox } from "./graphicsOverlay.ts";
export { GRAPHICS_PANEL_ID } from "../../app/useGraphics";
export type { GraphicOperations } from "../../app/useGraphics";
