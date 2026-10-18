import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { RecoveryStatus } from "./projectTypes";
import { useProject } from "./useProject";

export function useRecovery() {
  const [recoveryStatus, setRecoveryStatus] = useState<RecoveryStatus>({ available: false });
  const { setConnection } = useProject();

  useEffect(() => {
    let active = true;
    invoke<RecoveryStatus>("recovery_status")
      .then((recovery) => {
        if (!active) return;
        setRecoveryStatus(recovery);
        setConnection("Rust core connected");
      })
      .catch((error: unknown) => {
        if (active) setConnection(`Rust core error: ${String(error)}`);
      });
    return () => { active = false; };
  }, [setConnection]);

  return recoveryStatus;
}
