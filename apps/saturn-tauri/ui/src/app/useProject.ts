// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import { useContext } from "react";
import { ProjectContext } from "./ProjectProvider";

export function useProject() {
  const project = useContext(ProjectContext);
  if (!project) throw new Error("useProject must be used within ProjectProvider");
  return project;
}
