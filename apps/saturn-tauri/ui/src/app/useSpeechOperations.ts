import { useCallback, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ProjectDocument, SilenceInterval } from "./projectTypes";

/** Word-timed speech segments as produced by the transcribe wrapper. */
type SpeechSegmentShape = {
  id: number;
  start_ticks: number;
  end_ticks: number;
  text: string;
  speaker?: string | null;
  confidence: number;
  words: Array<{
    word: string;
    start_ticks: number;
    end_ticks: number;
    confidence: number;
    is_filler: boolean;
  }>;
};

export interface UseSpeechOperationsOptions {
  project: ProjectDocument["project"] | null;
  refreshProject: () => Promise<unknown>;
  setNotice: (message: string) => void;
  setBusy: (busy: boolean) => void;
}

/**
 * Speech-to-subtitles, silence detection/removal, and filler-word removal:
 * the transcription and speech-modals state plus the Tauri wrappers around
 * the engine's `speech.*` commands. EditorModals renders the two modals from
 * this state; the Audio > Speech and Edit > Speech menu ids open them.
 */
export function useSpeechOperations({
  project,
  refreshProject,
  setNotice,
  setBusy,
}: UseSpeechOperationsOptions) {
  // Transcribe modal
  const [transcribeModalOpen, setTranscribeModalOpen] = useState(false);
  const [transcribeLanguage, setTranscribeLanguage] = useState("en");
  const [transcribeModel, setTranscribeModel] = useState("base");
  const [transcribeTextOverride, setTranscribeTextOverride] = useState("");
  const [transcribeReplaceExisting, setTranscribeReplaceExisting] = useState(false);
  const [isTranscribing, setIsTranscribing] = useState(false);

  // Silence removal modal
  const [silenceModalOpen, setSilenceModalOpen] = useState(false);
  const [silenceThresholdDb, setSilenceThresholdDb] = useState(-35);
  const [silenceMinDurationMs, setSilenceMinDurationMs] = useState(400);
  const [silencePaddingMs, setSilencePaddingMs] = useState(50);
  const [silenceIntervals, setSilenceIntervals] = useState<SilenceInterval[]>([]);
  const [isDetectingSilence, setIsDetectingSilence] = useState(false);
  // Word-timed segments from the latest transcription; the filler-removal
  // flow consumes them so both features agree on what was heard.
  const lastTranscriptSegments = useRef<SpeechSegmentShape[]>([]);

  // Waveform peaks for the first media asset that carries one — the
  // analysis input for silence detection and silence ripple deletion.
  const activePeaks = useCallback((): number[] | null => {
    const assets = project?.media ?? [];
    const found = assets.find((asset) => Array.isArray(asset.waveform) && asset.waveform.length > 0);
    return found ? (found.waveform as number[]) : null;
  }, [project]);

  const handleAutoTranscribe = useCallback(async () => {
    setIsTranscribing(true);
    setNotice("Transcribing timeline audio with Whisper AI…");
    try {
      const result = await invoke<{
        subtitles: Array<{ id: number; text: string }>;
        segments: SpeechSegmentShape[];
      }>("transcribe_audio_speech", {
        options: {
          whisperModel: transcribeModel,
          language: transcribeLanguage === "auto" ? null : transcribeLanguage,
          textOverride: transcribeTextOverride || null,
          replaceExisting: transcribeReplaceExisting,
        },
      });
      lastTranscriptSegments.current = result.segments ?? [];
      await refreshProject();
      setTranscribeModalOpen(false);
      setNotice(
        `Auto-transcription complete — ${result.subtitles.length} subtitle cues added.`,
      );
    } catch (err) {
      setNotice(`Transcription failed: ${String(err)}`);
    } finally {
      setIsTranscribing(false);
    }
  }, [transcribeModel, transcribeLanguage, transcribeTextOverride, transcribeReplaceExisting, setNotice, refreshProject]);

  const handleDetectSilence = useCallback(async () => {
    const peaks = activePeaks();
    if (!peaks) {
      setNotice("Import a media asset with audio first — silence analysis needs its waveform");
      return;
    }
    setIsDetectingSilence(true);
    setNotice("Analyzing audio waveform for silence gaps…");
    try {
      const intervals = await invoke<SilenceInterval[]>("detect_timeline_silence", {
        peaks,
        thresholdDb: silenceThresholdDb,
        minDurationMs: silenceMinDurationMs,
      });
      setSilenceIntervals(intervals);
      setNotice(`Detected ${intervals.length} silence intervals across timeline.`);
    } catch (err) {
      setNotice(`Silence detection failed: ${String(err)}`);
    } finally {
      setIsDetectingSilence(false);
    }
  }, [activePeaks, silenceThresholdDb, silenceMinDurationMs, setNotice]);

  const handleRippleDeleteSilence = useCallback(async () => {
    const peaks = activePeaks();
    if (!peaks) {
      setNotice("Import a media asset with audio first — silence ripple needs its waveform");
      return;
    }
    setBusy(true);
    setNotice("Ripple-deleting silence gaps…");
    try {
      const report = await invoke<{ deleted_ranges_count: number }>("ripple_delete_silence", {
        peaks,
        thresholdDb: silenceThresholdDb,
        minDurationMs: silenceMinDurationMs,
        paddingMs: silencePaddingMs,
      });
      await refreshProject();
      setSilenceModalOpen(false);
      setNotice(
        `Ripple-removed ${report.deleted_ranges_count} silence pause${report.deleted_ranges_count === 1 ? "" : "s"} from the timeline`,
      );
    } catch (err) {
      setNotice(`Ripple delete failed: ${String(err)}`);
    } finally {
      setBusy(false);
    }
  }, [activePeaks, silenceThresholdDb, silenceMinDurationMs, silencePaddingMs, setBusy, setNotice, refreshProject]);

  const handleRippleDeleteFillers = useCallback(async () => {
    if (lastTranscriptSegments.current.length === 0) {
      setNotice("Transcribe the timeline first — filler removal works on the latest transcript");
      return;
    }
    setBusy(true);
    setNotice("Ripple-deleting verbal hesitation fillers…");
    try {
      const report = await invoke<{ deleted_ranges_count: number }>("ripple_delete_fillers", {
        segments: lastTranscriptSegments.current,
        customFillers: null,
      });
      await refreshProject();
      setSilenceModalOpen(false);
      setNotice(
        `Removed ${report.deleted_ranges_count} filler range${report.deleted_ranges_count === 1 ? "" : "s"} from the timeline`,
      );
    } catch (err) {
      setNotice(`Filler removal failed: ${String(err)}`);
    } finally {
      setBusy(false);
    }
  }, [setBusy, setNotice, refreshProject]);

  return {
    transcribeModalOpen,
    setTranscribeModalOpen,
    transcribeLanguage,
    setTranscribeLanguage,
    transcribeModel,
    setTranscribeModel,
    transcribeTextOverride,
    setTranscribeTextOverride,
    transcribeReplaceExisting,
    setTranscribeReplaceExisting,
    isTranscribing,
    silenceModalOpen,
    setSilenceModalOpen,
    silenceThresholdDb,
    setSilenceThresholdDb,
    silenceMinDurationMs,
    setSilenceMinDurationMs,
    silencePaddingMs,
    setSilencePaddingMs,
    silenceIntervals,
    isDetectingSilence,
    handleAutoTranscribe,
    handleDetectSilence,
    handleRippleDeleteSilence,
    handleRippleDeleteFillers,
  };
}
