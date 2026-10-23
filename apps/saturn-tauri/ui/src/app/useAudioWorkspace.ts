import { useCallback, useEffect, useMemo, useState } from "react";
import type { ProjectDocument, TrackAudioProcessing, BusTarget, AutomationMode, LoudnessStandard, LoudnessReport } from "./projectTypes";
import { audioClockMaster } from "../engine/playbackEngine";

const DEFAULT_DSP: TrackAudioProcessing = {
  eq_enabled: false,
  eq_low_gain: 0,
  eq_mid_gain: 0,
  eq_high_gain: 0,
  comp_enabled: false,
  comp_threshold: -24,
  comp_ratio: 4,
  comp_attack: 0.005,
  comp_release: 0.1,
  denoise_enabled: false,
  denoise_amount: 0.5,
};

const EMPTY_LOUDNESS: LoudnessReport = {
  integratedLufs: -70,
  shortTermLufs: -70,
  momentaryLufs: -70,
  loudnessRangeLu: 0,
  truePeakDbtp: -60,
  targetLufs: -23,
  isCompliant: false,
  gainRecommendationDb: 0,
};

type Perform = (id: string, params?: unknown, msg?: string) => Promise<boolean>;

export function useAudioWorkspace(
  project: ProjectDocument["project"] | null,
  perform: Perform,
  programPlaying: boolean,
) {
  const tracks = project?.sequence.tracks ?? [];
  const [selectedFairlightTrackId, setSelectedFairlightTrackId] = useState<number | null>(null);
  const [audioProcessingDrafts, setAudioProcessingDrafts] = useState<Record<number, TrackAudioProcessing>>({});
  const [masterGainDraft, setMasterGainDraftState] = useState(() => {
    try { return Number(localStorage.getItem("eep-master-gain") ?? 0); } catch { return 0; }
  });
  const [busGainDrafts, setBusGainDrafts] = useState<Record<string, number>>({});
  const [loudnessStandard, setLoudnessStandard] = useState<LoudnessStandard>("ebu_r128");
  const [vuLevels, setVuLevels] = useState<Record<number, { left: number; right: number; peakDb: number }>>({});
  const [masterVu, setMasterVu] = useState({ left: 0, right: 0, peakDb: -60 });
  const [loudnessReport, setLoudnessReport] = useState<LoudnessReport>(EMPTY_LOUDNESS);
  const [trackGr, setTrackGr] = useState<Record<number, number>>({});
  const [masterGr, setMasterGr] = useState(0);

  useEffect(() => {
    if (!project) return;
    setBusGainDrafts(Object.fromEntries((project.sequence.buses ?? []).map((bus) => [bus.target, bus.gain_db])));
    audioClockMaster.setMasterGain(0);
    audioClockMaster.updateTracksState(tracks, project.sequence.buses ?? []);
    const masterBusGain = project.sequence.buses?.find((bus) => bus.target === "master")?.gain_db;
    if (masterBusGain !== undefined) setMasterGainDraftState(masterBusGain);
    audioClockMaster.setLoudnessStandard(loudnessStandard);
  }, [project?.sequence.id, project?.sequence.tracks, project?.sequence.buses, loudnessStandard]);

  useEffect(() => {
    if (!project) return;
    let frame = 0;
    let lastUpdate = 0;
    const poll = (now: number) => {
      if (now - lastUpdate > 80) {
        lastUpdate = now;
        setVuLevels(Object.fromEntries(tracks.map((track) => [track.id, audioClockMaster.getTrackLevels(track.id)])));
        setMasterVu(audioClockMaster.getMasterLevels());
        setLoudnessReport(audioClockMaster.getLoudnessReport(loudnessStandard));
        setTrackGr(Object.fromEntries(tracks.map((track) => [track.id, audioClockMaster.getTrackGainReduction(track.id)])));
        setMasterGr(audioClockMaster.getMasterGainReduction());
      }
      frame = requestAnimationFrame(poll);
    };
    frame = requestAnimationFrame(poll);
    return () => cancelAnimationFrame(frame);
  }, [project, tracks, programPlaying, loudnessStandard]);

  const saveTrackAudioProcessing = useCallback(async (trackId: number, patch: Partial<TrackAudioProcessing>) => {
    const track = tracks.find((item) => item.id === trackId);
    if (!track) return;
    const updated = { ...DEFAULT_DSP, ...track.audio_processing, ...audioProcessingDrafts[trackId], ...patch };
    const ok = await perform("track.set_audio_processing", { track_id: trackId, ...updated });
    if (ok) setAudioProcessingDrafts((prev) => ({ ...prev, [trackId]: updated }));
  }, [tracks, audioProcessingDrafts, perform]);

  const setMasterGain = useCallback((gainDb: number) => {
    setMasterGainDraftState(gainDb);
    audioClockMaster.setBusGain("master", gainDb);
    try { localStorage.setItem("eep-master-gain", String(gainDb)); } catch { /* storage can be disabled */ }
  }, []);

  const saveMasterGain = useCallback(async (gainDb: number) => {
    setMasterGain(gainDb);
    await perform("bus.set_gain_db", { target: "master", gain_db: gainDb });
  }, [perform, setMasterGain]);

  const setBusGain = useCallback(async (target: BusTarget, gainDb: number) => {
    setBusGainDrafts((prev) => ({ ...prev, [target]: gainDb }));
    audioClockMaster.setBusGain(target, gainDb);
    await perform("bus.set_gain_db", { target, gain_db: gainDb });
  }, [perform]);

  const clearAllSolos = useCallback(async () => {
    await Promise.all(tracks.filter((track) => track.solo).map((track) => perform("track.set_solo", { track_id: track.id, solo: false })));
  }, [tracks, perform]);

  const muteAllTracks = useCallback(async () => {
    const shouldMute = tracks.some((track) => !track.muted);
    await Promise.all(tracks.filter((track) => Boolean(track.muted) !== shouldMute).map((track) => perform("track.set_muted", { track_id: track.id, muted: shouldMute })));
  }, [tracks, perform]);

  const setTrackBusRoute = useCallback(async (trackId: number, bus: BusTarget) => {
    await perform("track.set_bus_route", { track_id: trackId, bus });
  }, [perform]);

  const setTrackAutomationMode = useCallback(async (trackId: number, mode: AutomationMode) => {
    const track = tracks.find((item) => item.id === trackId);
    if (!track) return;
    await perform("track.set_automation", {
      track_id: trackId,
      automation: { mode, volume_points: track.automation?.volume_points ?? [], pan_points: track.automation?.pan_points ?? [] },
    });
  }, [tracks, perform]);

  const autoTrimLoudness = useCallback(() => {
    const target = audioClockMaster.applyLoudnessAutoTrim(loudnessStandard);
    if (target !== 0) void saveMasterGain(target);
  }, [loudnessStandard, saveMasterGain]);

  const resetLoudness = useCallback(() => {
    audioClockMaster.resetLoudness();
    setLoudnessReport(audioClockMaster.getLoudnessReport(loudnessStandard));
  }, [loudnessStandard]);

  return useMemo(() => ({
    selectedFairlightTrackId, setSelectedFairlightTrackId,
    audioProcessingDrafts, setAudioProcessingDrafts, saveTrackAudioProcessing,
    trackGr, masterGr, vuLevels, masterVu,
    masterGainDraft, setMasterGainDraft: setMasterGain, saveMasterGain,
    busGainDrafts, setBusGainDrafts, setBusGain,
    loudnessStandard, setLoudnessStandard, loudnessReport, autoTrimLoudness, resetLoudness,
    clearAllSolos, muteAllTracks, setTrackBusRoute, setTrackAutomationMode,
  }), [selectedFairlightTrackId, audioProcessingDrafts, saveTrackAudioProcessing, trackGr, masterGr, vuLevels, masterVu,
    masterGainDraft, setMasterGain, saveMasterGain, busGainDrafts, setBusGain, loudnessStandard, loudnessReport,
    autoTrimLoudness, resetLoudness, clearAllSolos, muteAllTracks, setTrackBusRoute, setTrackAutomationMode]);
}
