// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import { useEffect, useMemo, useState, type ReactNode } from "react";
import {
  FolderKanban,
  Scissors,
  Workflow,
  Palette,
  Volume2,
  Sparkles,
  Spline,
  Send,
  Upload,
  Save,
  History,
} from "lucide-react";
import {
  Breadcrumb,
  Sidebar,
  StatusPill,
  type SidebarSection,
} from "../components/linear";
import type { EditorWorkspace } from "../app/useShortcuts";
import { editorWorkspaces } from "../features/menubar/AppHeader";

export interface EditorShellFrameProps {
  projectName: string;
  sequenceName: string;
  activeWorkspace: EditorWorkspace;
  onSelectWorkspace: (ws: EditorWorkspace) => void;
  onImportMedia: () => void;
  onSaveProject: () => void;
  onOpenHistory: () => void;
  busy: boolean;
  connection: string;
  children: ReactNode;
}

const WORKSPACE_ICONS: Record<string, ReactNode> = {
  media: <FolderKanban className="size-4" />,
  edit: <Scissors className="size-4" />,
  fusion: <Workflow className="size-4" />,
  color: <Palette className="size-4" />,
  audio: <Volume2 className="size-4" />,
  effects: <Sparkles className="size-4" />,
  keyframes: <Spline className="size-4" />,
  deliver: <Send className="size-4" />,
};

export function EditorShellFrame({
  projectName,
  sequenceName,
  activeWorkspace,
  onSelectWorkspace,
  onImportMedia,
  onSaveProject,
  onOpenHistory,
  busy,
  connection,
  children,
}: EditorShellFrameProps) {
  const [userCollapsed, setUserCollapsed] = useState<boolean | null>(null);
  const [narrowWindow, setNarrowWindow] = useState(() =>
    typeof window !== "undefined" ? window.innerWidth <= 1160 : false,
  );

  useEffect(() => {
    const onResize = () => {
      setNarrowWindow(window.innerWidth <= 1160);
    };
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);

  const collapsed = userCollapsed !== null ? userCollapsed : narrowWindow;

  const sections: SidebarSection[] = useMemo(
    () => [
      {
        title: "Workspaces",
        items: editorWorkspaces.map((ws) => ({
          id: ws.id,
          label: ws.label,
          icon: WORKSPACE_ICONS[ws.id] ?? <Scissors className="size-4" />,
          badge: ws.isExperimental ? "EXP" : undefined,
          shortcut: ws.shortcut,
        })),
      },
      {
        title: "Project",
        items: [
          {
            id: "__action_import",
            label: "Import Media",
            icon: <Upload className="size-4" />,
            shortcut: "Ctrl+I",
          },
          {
            id: "__action_save",
            label: "Save Project",
            icon: <Save className="size-4" />,
            shortcut: "Ctrl+S",
          },
          {
            id: "__action_history",
            label: "Undo History",
            icon: <History className="size-4" />,
          },
        ],
      },
    ],
    [],
  );

  const handleSelect = (id: string) => {
    if (id === "__action_import") {
      onImportMedia();
      return;
    }
    if (id === "__action_save") {
      onSaveProject();
      return;
    }
    if (id === "__action_history") {
      onOpenHistory();
      return;
    }
    onSelectWorkspace(id as EditorWorkspace);
  };

  const activeWsLabel =
    editorWorkspaces.find((w) => w.id === activeWorkspace)?.label ?? "Edit";

  return (
    <div className="flex-1 min-h-0 flex overflow-hidden">
      <Sidebar
        sections={sections}
        activeId={activeWorkspace}
        onSelect={handleSelect}
        collapsed={collapsed}
        onToggleCollapse={() => setUserCollapsed((prev) => !(prev ?? narrowWindow))}
        header={
          <span className="text-xs font-semibold text-[var(--text)] truncate">
            {projectName}
          </span>
        }
        footer={
          !collapsed ? (
            <StatusPill
              label="Core"
              status={busy ? "active" : "online"}
              monoValue={connection}
              className="w-full justify-between"
            />
          ) : undefined
        }
      />
      <div className="flex-1 min-w-0 min-h-0 flex flex-col overflow-hidden">
        <Breadcrumb
          segments={[
            { label: "Easy Edits Pro", onClick: () => onSelectWorkspace("edit") },
            { label: projectName },
            { label: activeWsLabel, active: true },
          ]}
          right={
            <StatusPill
              label="Sequence"
              status="active"
              monoValue={sequenceName}
            />
          }
        />
        <div className="app-workspace-region flex-1 min-w-0 min-h-0">
          {children}
        </div>
      </div>
    </div>
  );
}
