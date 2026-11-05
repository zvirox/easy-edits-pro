// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import { InteriorButton } from "../../components/interior/interior-button";
import { GlassDialog } from "../../motion/GlassDialog";
export interface NewProjectModalProps {
  isOpen: boolean;
  onClose: () => void;
  name: string;
  setName: (name: string) => void;
  onCreate: () => Promise<void>;
}

export function NewProjectModal({
  isOpen,
  onClose,
  name,
  setName,
  onCreate,
}: NewProjectModalProps) {
  return (
    <GlassDialog
      as="form"
      open={isOpen}
      cardClassName="new-project-dialog glass"
      cardProps={{
        onSubmit: (event) => {
          event.preventDefault();
          void onCreate();
        },
      }}
    >
      <h2>New project</h2>
      <label>
        Project name
        <input
          autoFocus
          value={name}
          onChange={(event) => setName(event.target.value)}
        />
      </label>
      <div className="dialog-actions">
        <InteriorButton type="button" onClick={onClose}>
          Cancel
        </InteriorButton>
        <InteriorButton className="primary" type="submit" disabled={!name.trim()}>
          Create
        </InteriorButton>
      </div>
    </GlassDialog>
  );
}
