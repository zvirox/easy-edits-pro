import { InteriorButton } from "../../components/interior/interior-button";
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
  if (!isOpen) return null;

  return (
    <div className="dialog-backdrop" role="presentation">
      <form
        className="new-project-dialog glass"
        data-elev="4"
        onSubmit={(event) => {
          event.preventDefault();
          void onCreate();
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
      </form>
    </div>
  );
}
