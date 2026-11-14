// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import { createContext, useContext } from "react";

export interface DockDrawerContextValue {
  isDrawerOpen: boolean;
  closeDrawer: () => void;
  openDrawer: () => void;
  toggleDrawer: () => void;
}

export const DockDrawerContext = createContext<DockDrawerContextValue>({
  isDrawerOpen: false,
  closeDrawer: () => {},
  openDrawer: () => {},
  toggleDrawer: () => {},
});

export const useDockDrawer = () => useContext(DockDrawerContext);
