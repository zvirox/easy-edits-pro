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
        className="new-project-dialog"
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
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button className="primary" type="submit" disabled={!name.trim()}>
            Create
          </button>
        </div>
      </form>
    </div>
  );
}
