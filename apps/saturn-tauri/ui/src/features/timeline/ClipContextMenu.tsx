import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { InteriorButton } from "../../components/interior/interior-button";
import { formatShortcut } from "../shortcuts/shortcuts";

export interface ClipMenuCommand {
  id: string;
  label: string;
  menu: string[];
  shortcut: string | null;
  shipped?: boolean;
}

export interface ClipContextMenuProps {
  x: number;
  y: number;
  commands: ClipMenuCommand[];
  disabledReasons: Record<string, string | null>;
  onSelect: (id: string) => void;
  onClose: () => void;
}

const MENU_WIDTH = 230;
const MENU_ITEM_HEIGHT = 30;
const MENU_PADDING = 10;

/** Right-click clip menu. Labels, order, and shortcuts come from the catalogue. */
export function ClipContextMenu({
  x,
  y,
  commands,
  disabledReasons,
  onSelect,
  onClose,
}: ClipContextMenuProps) {
  const menuRef = useRef<HTMLDivElement | null>(null);
  const [position, setPosition] = useState({ left: x, top: y });

  useEffect(() => {
    const height = commands.length * MENU_ITEM_HEIGHT + MENU_PADDING;
    setPosition({
      left: Math.max(8, Math.min(x, window.innerWidth - MENU_WIDTH - 8)),
      top: Math.max(8, Math.min(y, window.innerHeight - height - 8)),
    });
  }, [x, y, commands.length]);

  useEffect(() => {
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    const closeOnOutsidePointer = (event: globalThis.PointerEvent) => {
      if (
        menuRef.current &&
        !menuRef.current.contains(event.target as Node)
      ) {
        onClose();
      }
    };
    const closeOnResize = () => onClose();
    window.addEventListener("keydown", closeOnEscape);
    window.addEventListener("pointerdown", closeOnOutsidePointer);
    window.addEventListener("resize", closeOnResize);
    return () => {
      window.removeEventListener("keydown", closeOnEscape);
      window.removeEventListener("pointerdown", closeOnOutsidePointer);
      window.removeEventListener("resize", closeOnResize);
    };
  }, [onClose]);

  return createPortal(
    <div
      ref={menuRef}
      className="clip-context-menu glass"
      data-elev="3"
      role="menu"
      style={{ left: `${position.left}px`, top: `${position.top}px` }}
    >
      {commands.map((command) => {
        const reason = disabledReasons[command.id] ?? null;
        return (
          <InteriorButton
            key={command.id}
            type="button"
            role="menuitem"
            className="clip-context-menu-item"
            title={reason ?? undefined}
            aria-disabled={reason !== null}
            disabled={reason !== null}
            onClick={() => {
              onClose();
              onSelect(command.id);
            }}
          >
            <span>{command.label}</span>
            {command.shortcut && <small>{formatShortcut(command.shortcut)}</small>}
          </InteriorButton>
        );
      })}
    </div>,
    document.body
  );
}
