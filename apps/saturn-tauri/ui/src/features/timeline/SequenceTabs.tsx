import { InteriorButton } from "../../components/interior/interior-button";

export interface SequenceTab {
  id: number;
  name: string;
  isCompound: boolean;
}

export interface SequenceTabsProps {
  sequences: SequenceTab[];
  activeId: number;
  busy: boolean;
  onSelect: (sequenceId: number) => void;
  onCreate: () => void;
  onDuplicate: () => void;
  onRename: () => void;
  onDelete: () => void;
}

/**
 * Sequence tabs are the authoritative sequence navigator. Compound sequences
 * are excluded unless they are active, so the strip stays focused on user
 * sequences while still letting an open compound be exited.
 */
export function SequenceTabs({
  sequences,
  activeId,
  busy,
  onSelect,
  onCreate,
  onDuplicate,
  onRename,
  onDelete,
}: SequenceTabsProps) {
  const active = sequences.find((sequence) => sequence.id === activeId);
  const lastSequence = sequences.length <= 1;

  return (
    <div className="sequence-tabs" aria-label="Sequences">
      <div className="sequence-tabs-scroll" role="tablist" aria-label="Project sequences">
        {sequences.map((sequence) => {
          const isActive = sequence.id === activeId;
          return (
            <button
              key={sequence.id}
              type="button"
              role="tab"
              aria-selected={isActive}
              className={`sequence-tab ${isActive ? "active" : ""}`}
              onClick={() => {
                if (!isActive) onSelect(sequence.id);
              }}
              onDoubleClick={() => {
                if (isActive) onRename();
              }}
              title={
                sequence.isCompound
                  ? `${sequence.name} — compound sequence`
                  : `${sequence.name} — double-click to rename`
              }
            >
              <span className="sequence-tab-icon" aria-hidden="true">
                {sequence.isCompound ? "⧉" : "▤"}
              </span>
              <span className="sequence-tab-name">{sequence.name}</span>
              {isActive && <span className="sequence-tab-active-dot" aria-hidden="true" />}
            </button>
          );
        })}
      </div>
      <div className="sequence-tabs-actions">
        <InteriorButton
          type="button"
          className="sequence-tab-action"
          disabled={busy}
          onClick={onCreate}
          data-tooltip="New Sequence"
          data-tooltip-pos="top"
          aria-label="New Sequence"
        >
          ＋
        </InteriorButton>
        <InteriorButton
          type="button"
          className="sequence-tab-action"
          disabled={busy || !active}
          onClick={onDuplicate}
          data-tooltip="Duplicate Active Sequence"
          data-tooltip-pos="top"
          aria-label="Duplicate Active Sequence"
        >
          ⧉
        </InteriorButton>
        <InteriorButton
          type="button"
          className="sequence-tab-action"
          disabled={busy || !active}
          onClick={onRename}
          data-tooltip="Rename Active Sequence"
          data-tooltip-pos="top"
          aria-label="Rename Active Sequence"
        >
          ✎
        </InteriorButton>
        <InteriorButton
          type="button"
          className="sequence-tab-action is-danger"
          disabled={busy || lastSequence || active?.isCompound}
          onClick={onDelete}
          data-tooltip={
            lastSequence
              ? "A project must keep one sequence"
              : "Delete Active Sequence"
          }
          data-tooltip-pos="top"
          aria-label="Delete Active Sequence"
        >
          🗑
        </InteriorButton>
      </div>
    </div>
  );
}
