import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { HardwareEncoderInfo } from "../types";

export type JobState =
  | { status: "queued" }
  | { status: "running"; progress: number; phase: string }
  | { status: "completed"; output?: string | null }
  | { status: "failed"; error: string }
  | { status: "cancelled" };

export interface JobInfo {
  id: number;
  label: string;
  kind: { type: string; [key: string]: unknown };
  priority: "background" | "normal" | "high" | "immediate";
  state: JobState;
  created_at: number;
  started_at?: number | null;
  completed_at?: number | null;
}

export function useJobs() {
  const [jobs, setJobs] = useState<JobInfo[]>([]);
  const [jobDrawerOpen, setJobDrawerOpen] = useState(false);
  const jobDrawerRef = useRef<HTMLDivElement>(null);
  const activeJobs = useMemo(() => jobs.filter((job) => job.state.status === "queued" || job.state.status === "running"), [jobs]);

  useEffect(() => {
    invoke<JobInfo[]>("get_all_jobs")
      .then((data) => { if (Array.isArray(data)) setJobs(data); })
      .catch((error: unknown) => console.warn("Could not fetch jobs:", error));
    const unlisten = listen<JobInfo>("job_queue_updated", ({ payload }) => {
      setJobs((previous) => {
        const index = previous.findIndex((job) => job.id === payload.id);
        if (index < 0) return [payload, ...previous];
        return previous.map((job) => job.id === payload.id ? payload : job);
      });
    });
    return () => { void unlisten.then((stop) => stop()); };
  }, []);

  useEffect(() => {
    if (!jobDrawerOpen) return;
    const handleClickOutside = (event: globalThis.MouseEvent) => {
      if (jobDrawerRef.current && !jobDrawerRef.current.contains(event.target as Node)) setJobDrawerOpen(false);
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [jobDrawerOpen]);

  const cancelJob = useCallback(async (jobId: number) => {
    try { await invoke("cancel_job", { id: jobId }); }
    catch (error) { console.warn("Failed to cancel job:", error); }
  }, []);
  const cancelAllJobs = useCallback(async () => {
    try { await invoke("cancel_all_jobs"); }
    catch (error) { console.warn("Failed to cancel all jobs:", error); }
  }, []);
  const clearJobHistory = useCallback(async () => {
    try {
      await invoke("clear_job_history");
      setJobs((previous) => previous.filter((job) => job.state.status === "queued" || job.state.status === "running"));
    } catch (error) { console.warn("Failed to clear job history:", error); }
  }, []);

  return { jobs, setJobs, jobDrawerOpen, setJobDrawerOpen, jobDrawerRef, activeJobs, cancelJob, cancelAllJobs, clearJobHistory };
}

/**
 * Hardware-encoder probe capabilities (F5). Startup probes on a background
 * job and caches the result process-wide, so this hook never blocks the UI:
 * it reads the cached answer through the existing command and follows the
 * `encoder_capabilities_ready` event in case the probe is still running.
 */
export function useEncoderCapabilities(): HardwareEncoderInfo | null {
  const [info, setInfo] = useState<HardwareEncoderInfo | null>(null);
  useEffect(() => {
    let cancelled = false;
    invoke<HardwareEncoderInfo>("detect_encoder_capabilities")
      .then((data) => {
        if (!cancelled && data) setInfo(data);
      })
      .catch((error: unknown) => console.warn("Encoder probe failed:", error));
    const unlisten = listen<HardwareEncoderInfo>("encoder_capabilities_ready", ({ payload }) => {
      if (!cancelled && payload) setInfo(payload);
    });
    return () => {
      cancelled = true;
      void unlisten.then((stop) => stop());
    };
  }, []);
  return info;
}
