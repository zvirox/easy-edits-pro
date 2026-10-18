import type { CSSProperties } from "react";

export const TICKS_PER_SECOND = 254_016_000_000;

export type TransitionKind =
  | "crossfade"
  | "dip_to_black"
  | "dip_to_white"
  | "wipe_left"
  | "wipe_right"
  | "slide_left"
  | "slide_right";

export interface Transition {
  id: number;
  kind: TransitionKind;
  cut_at: number;
  duration: number;
}

export type KeyframeProperty = "position_x" | "position_y" | "scale" | "rotation" | "opacity";

export type InterpolationKind = "linear" | "ease_in" | "ease_out" | "ease_in_out" | "bezier";

export interface Keyframe {
  property: KeyframeProperty;
  time: number;
  value: number;
  interpolation?: InterpolationKind;
  handle_in_x?: number | null;
  handle_in_y?: number | null;
  handle_out_x?: number | null;
  handle_out_y?: number | null;
}

export interface ColorWheelParam {
  red: number;
  green: number;
  blue: number;
  luma: number;
}

export interface ColorWheels {
  lift: ColorWheelParam;
  gamma: ColorWheelParam;
  gain: ColorWheelParam;
  offset: ColorWheelParam;
}

export interface CurvePoint {
  x: number;
  y: number;
}

export interface RgbCurves {
  master: CurvePoint[];
  red: CurvePoint[];
  green: CurvePoint[];
  blue: CurvePoint[];
}

export interface HslQualifier {
  enabled: boolean;
  hue_center: number;
  hue_width: number;
  hue_softness: number;
  sat_center: number;
  sat_width: number;
  sat_softness: number;
  luma_center: number;
  luma_width: number;
  luma_softness: number;
  invert: boolean;
  blur_radius: number;
  hue_shift: number;
  sat_boost: number;
  exposure_shift: number;
}

export interface TrackGroup {
  id: number;
  name: string;
  color: string;
  collapsed: boolean;
  muted: boolean;
  solo: boolean;
  locked: boolean;
}

export interface ColorAdjustments {
  exposure: number;
  contrast: number;
  saturation: number;
  hue?: number;
  wheels?: ColorWheels;
  curves?: RgbCurves;
  qualifier?: HslQualifier;
}

export type MultiCamSyncMode = "timecode" | "audio_waveform" | "marker";

export interface MultiCamAngle {
  id: number;
  name: string;
  media_id: number;
  sync_offset_ticks: number;
}

export interface MultiCamClip {
  id: number;
  name: string;
  angles: MultiCamAngle[];
  active_angle_id: number;
  sync_mode?: MultiCamSyncMode;
}

export type TrackingMode = "transform" | "stabilize" | "pin";

export interface TrackPoint {
  time: number;
  x: number;
  y: number;
  confidence: number;
}

export interface MotionTrackingData {
  id: number;
  name: string;
  mode: TrackingMode;
  points: TrackPoint[];
  pinned_clip_id?: number | null;
  smoothness: number;
}

export type BusTarget = "master" | "submix1" | "submix2" | "aux_fx";
export type AutomationMode = "off" | "read" | "touch" | "latch" | "write";

export interface AutomationPoint {
  time: number;
  value: number;
}

export interface TrackAutomation {
  mode: AutomationMode;
  volume_points: AutomationPoint[];
  pan_points: AutomationPoint[];
}

export interface AudioPluginInstance {
  id: number;
  name: string;
  plugin_format: string;
  enabled: boolean;
  preset: string;
  params: Record<string, number>;
}

export interface AudioBus {
  target: BusTarget;
  name: string;
  gain_db: number;
  muted: boolean;
  solo: boolean;
  processing: TrackAudioProcessing;
}

export type EffectKind =
  | "color_correction"
  | "gaussian_blur"
  | "crop"
  | "vignette"
  | "chroma_key"
  | "lut"
  | "custom_shader";

export interface ColorCorrectionParams {
  exposure: number;
  contrast: number;
  saturation: number;
  hue: number;
  wheels?: ColorWheels;
}

export interface GaussianBlurParams {
  sigma: number;
}

export interface CropParams {
  top: number;
  bottom: number;
  left: number;
  right: number;
}

export interface VignetteParams {
  intensity: number;
  radius: number;
  softness: number;
}

export interface ChromaKeyParams {
  color: string;
  tolerance: number;
  softness: number;
  spill_suppression: number;
}

export interface LutParams {
  name: string;
  path?: string | null;
  intensity: number;
  preset_id?: string | null;
}

export interface CustomShaderParams {
  name: string;
  shader_code: string;
  preset_id?: string | null;
  uniforms?: Record<string, number>;
}

export type EffectPayload =
  | { kind: "color_correction"; params: ColorCorrectionParams }
  | { kind: "gaussian_blur"; params: GaussianBlurParams }
  | { kind: "crop"; params: CropParams }
  | { kind: "vignette"; params: VignetteParams }
  | { kind: "chroma_key"; params: ChromaKeyParams }
  | { kind: "lut"; params: LutParams }
  | { kind: "custom_shader"; params: CustomShaderParams };

export interface EffectInstance {
  id: number;
  name: string;
  enabled: boolean;
  kind: EffectKind;
  params: ColorCorrectionParams | GaussianBlurParams | CropParams | VignetteParams | ChromaKeyParams | LutParams | CustomShaderParams;
}

export type CompositingNodeKind =
  | "merge"
  | "transform"
  | "keyer"
  | "mask"
  | "particles"
  | "media_source"
  | "output_sink";

export interface CompositingNode {
  id: number;
  kind: CompositingNodeKind;
  title: string;
  pos_x: number;
  pos_y: number;
  params?: Record<string, unknown>;
  enabled?: boolean;
}

export interface NodeConnection {
  from_node: number;
  from_socket: string;
  to_node: number;
  to_socket: string;
}

export interface RotoSplinePoint {
  x: number;
  y: number;
  handle_in?: [number, number];
  handle_out?: [number, number];
}

export interface RotoSplineMask {
  id: number;
  name: string;
  closed?: boolean;
  inverted?: boolean;
  feather?: number;
  opacity?: number;
  points: RotoSplinePoint[];
}

export interface NodeGraph {
  nodes: CompositingNode[];
  connections: NodeConnection[];
  masks?: RotoSplineMask[];
}

export interface SpeedRampPoint {
  time: number;
  speed_factor: number;
  bezier_tangent?: [number, number];
}

export interface SpeedRamp {
  points: SpeedRampPoint[];
  optical_flow_interpolation?: boolean;
  frame_blend?: boolean;
}

export interface MotionBlurParams {
  enabled?: boolean;
  shutter_angle_deg?: number;
  samples?: number;
}

export type LockTargetKind = "bin" | "sequence" | "track" | "clip";

export interface UserLock {
  user_id: string;
  user_name: string;
  user_color?: string;
  target_kind: LockTargetKind;
  target_id: number;
  locked_at: number;
}

export interface TimelineChangeEntry {
  id: number;
  user_id: string;
  user_name: string;
  timestamp: number;
  action: string;
  summary: string;
}

export interface CollaborationState {
  session_id: string;
  active_users: string[];
  locks: UserLock[];
  change_log: TimelineChangeEntry[];
}

export interface Clip {
  media_id: number;
  timeline_start: number;
  source_in: number;
  duration: number;
  speed?: number;
  group_id?: number | null;
  color: ColorAdjustments;
  effects?: EffectInstance[];
  keyframes: Keyframe[];
  multicam_id?: number | null;
  multicam_angle_id?: number | null;
  tracking?: MotionTrackingData | null;
  node_graph?: NodeGraph | null;
  speed_ramp?: SpeedRamp | null;
  motion_blur?: MotionBlurParams | null;
  compound_sequence_id?: number | null;
}

export interface TrackAudioProcessing {
  eq_enabled?: boolean;
  eq_low_gain?: number;
  eq_mid_gain?: number;
  eq_high_gain?: number;
  comp_enabled?: boolean;
  comp_threshold?: number;
  comp_ratio?: number;
  comp_attack?: number;
  comp_release?: number;
  denoise_enabled?: boolean;
  denoise_amount?: number;
}

export interface Track {
  id: number;
  group_id?: number | null;
  kind: "video" | "audio";
  name: string;
  gain_db: number;
  pan?: number;
  muted?: boolean;
  solo?: boolean;
  locked?: boolean;
  audio_processing?: TrackAudioProcessing;
  bus?: BusTarget;
  automation?: TrackAutomation;
  plugins?: AudioPluginInstance[];
  clips: Clip[];
  transitions?: Transition[];
}

export interface TitleProperties {
  text: string;
  subtitle?: string | null;
  style: string;
  font_family: string;
  font_size: number;
  color: string;
  bg_color?: string | null;
}

export interface Sequence {
  id?: number;
  name: string;
  track_groups?: TrackGroup[];
  tracks: Track[];
  mark_in: number | null;
  mark_out: number | null;
  subtitles?: SubtitleItem[];
  multicam_clips?: MultiCamClip[];
  buses?: AudioBus[];
  node_graph?: NodeGraph | null;
}

export interface MediaBin {
  id: number;
  name: string;
  parent_id?: number | null;
  color?: string | null;
}

export interface MediaAsset {
  id: number;
  name: string;
  path: string;
  kind: "video" | "audio" | "image" | "title" | "compound_sequence" | "unknown";
  duration: number | null;
  width: number | null;
  height: number | null;
  fps?: number | null;
  audio_channels?: number | null;
  codec?: string | null;
  bin_id?: number | null;
  color_label?: string | null;
  tags?: string[] | null;
  is_missing?: boolean;
  title?: TitleProperties | null;
  waveform?: number[] | null;
  nested_sequence?: Sequence | null;
}

export interface SubtitleItem {
  id: number;
  start_ticks: number;
  end_ticks: number;
  text: string;
}

export interface ProjectDocument {
  project: {
    name: string;
    media: MediaAsset[];
    bins?: MediaBin[];
    sequence: Sequence;
    sequences?: Sequence[];
    settings: {
      width: number;
      height: number;
      frame_rate: { numerator: number; denominator: number };
      audio_sample_rate: number;
    };
    collaboration?: CollaborationState;
  };
}

export type LoudnessStandard = "ebu_r128" | "streaming" | "atsc_a85";

export interface LoudnessReport {
  momentaryLufs: number;
  shortTermLufs: number;
  integratedLufs: number;
  loudnessRangeLu: number;
  truePeakDbtp: number;
  targetLufs: number;
  isCompliant: boolean;
  gainRecommendationDb: number;
}

/**
 * Direct Form II Transposed Biquad Filter for real-time DSP.
 */
class BiquadTransposed {
  b0: number;
  b1: number;
  b2: number;
  a1: number;
  a2: number;
  s1 = 0;
  s2 = 0;

  constructor(b0: number, b1: number, b2: number, a1: number, a2: number) {
    this.b0 = b0;
    this.b1 = b1;
    this.b2 = b2;
    this.a1 = a1;
    this.a2 = a2;
  }

  process(x: number): number {
    const y = this.b0 * x + this.s1;
    this.s1 = this.b1 * x - this.a1 * y + this.s2;
    this.s2 = this.b2 * x - this.a2 * y;
    return y;
  }

  reset(): void {
    this.s1 = 0;
    this.s2 = 0;
  }
}

/**
 * Real-time streaming EBU R128 & ITU-R BS.1770-4 Loudness Meter implementation
 * supporting EBU R128 (-23 LUFS), Online Streaming (-14 LUFS), and ATSC A/85 (-24 LUFS).
 */
export class EbuR128WebMeter {
  private sampleRate = 48000;
  private standard: LoudnessStandard = "ebu_r128";

  private stage1L: BiquadTransposed;
  private stage1R: BiquadTransposed;
  private stage2L: BiquadTransposed;
  private stage2R: BiquadTransposed;

  private momentaryRing: Float64Array;
  private momentaryPos = 0;
  private momentaryCount = 0;
  private momentarySum = 0;

  private shortTermRing: Float64Array;
  private shortTermPos = 0;
  private shortTermCount = 0;
  private shortTermSum = 0;

  private samplesIn100ms: number;
  private block100msSamples = 0;
  private block100msEnergySum = 0;
  private block100msHistory: number[] = [];
  private gatedBlocksEnergy: number[] = [];
  private shortTermHistory: number[] = [];
  private maxTruePeakLinear = 0;

  constructor(sampleRate = 48000, standard: LoudnessStandard = "ebu_r128") {
    this.sampleRate = sampleRate;
    this.standard = standard;

    // ITU-R BS.1770-4 48 kHz pre-filter coefficients
    this.stage1L = new BiquadTransposed(1.53512485958697, -2.69169618940638, 1.19839281085285, -1.69065929318241, 0.73248077421585);
    this.stage1R = new BiquadTransposed(1.53512485958697, -2.69169618940638, 1.19839281085285, -1.69065929318241, 0.73248077421585);
    // RLB high-pass filter coefficients
    this.stage2L = new BiquadTransposed(1.0, -2.0, 1.0, -1.99004745483398, 0.99007225036621);
    this.stage2R = new BiquadTransposed(1.0, -2.0, 1.0, -1.99004745483398, 0.99007225036621);

    const mLen = Math.max(1, Math.round(sampleRate * 0.4));
    const sLen = Math.max(1, Math.round(sampleRate * 3.0));
    this.samplesIn100ms = Math.max(1, Math.round(sampleRate * 0.1));

    this.momentaryRing = new Float64Array(mLen);
    this.shortTermRing = new Float64Array(sLen);
  }

  setStandard(std: LoudnessStandard): void {
    this.standard = std;
  }

  getStandard(): LoudnessStandard {
    return this.standard;
  }

  getSampleRate(): number {
    return this.sampleRate;
  }

  reset(): void {
    this.stage1L.reset();
    this.stage1R.reset();
    this.stage2L.reset();
    this.stage2R.reset();
    this.momentaryRing.fill(0);
    this.momentaryPos = 0;
    this.momentaryCount = 0;
    this.momentarySum = 0;
    this.shortTermRing.fill(0);
    this.shortTermPos = 0;
    this.shortTermCount = 0;
    this.shortTermSum = 0;
    this.block100msSamples = 0;
    this.block100msEnergySum = 0;
    this.block100msHistory = [];
    this.gatedBlocksEnergy = [];
    this.shortTermHistory = [];
    this.maxTruePeakLinear = 0;
  }

  processBuffer(left: Float32Array, right: Float32Array): void {
    const len = left.length;
    for (let i = 0; i < len; i++) {
      const inL = left[i];
      const inR = right[i];

      const absL = Math.abs(inL);
      const absR = Math.abs(inR);
      if (absL > this.maxTruePeakLinear) this.maxTruePeakLinear = absL;
      if (absR > this.maxTruePeakLinear) this.maxTruePeakLinear = absR;

      const y1L = this.stage1L.process(inL);
      const y1R = this.stage1R.process(inR);

      const y2L = this.stage2L.process(y1L);
      const y2R = this.stage2R.process(y1R);

      const energy = y2L * y2L + y2R * y2R;

      const oldM = this.momentaryRing[this.momentaryPos];
      this.momentaryRing[this.momentaryPos] = energy;
      this.momentarySum = Math.max(0, this.momentarySum - oldM + energy);
      this.momentaryPos = (this.momentaryPos + 1) % this.momentaryRing.length;
      if (this.momentaryCount < this.momentaryRing.length) this.momentaryCount++;

      const oldS = this.shortTermRing[this.shortTermPos];
      this.shortTermRing[this.shortTermPos] = energy;
      this.shortTermSum = Math.max(0, this.shortTermSum - oldS + energy);
      this.shortTermPos = (this.shortTermPos + 1) % this.shortTermRing.length;
      if (this.shortTermCount < this.shortTermRing.length) this.shortTermCount++;

      this.block100msEnergySum += energy;
      this.block100msSamples++;

      if (this.block100msSamples >= this.samplesIn100ms) {
        const subBlockMean = this.block100msEnergySum / this.block100msSamples;
        this.block100msHistory.push(subBlockMean);
        this.block100msEnergySum = 0;
        this.block100msSamples = 0;

        if (this.block100msHistory.length >= 4) {
          const hLen = this.block100msHistory.length;
          const blockEnergy = (
            this.block100msHistory[hLen - 4] +
            this.block100msHistory[hLen - 3] +
            this.block100msHistory[hLen - 2] +
            this.block100msHistory[hLen - 1]
          ) * 0.25;
          this.gatedBlocksEnergy.push(blockEnergy);

          const stMean = this.shortTermCount > 0 ? this.shortTermSum / this.shortTermCount : 0;
          if (stMean > 1e-12) {
            const stLufs = -0.691 + 10 * Math.log10(stMean);
            if (stLufs > -70.0) {
              this.shortTermHistory.push(stLufs);
            }
          }
        }
      }
    }
  }

  getReport(standard?: LoudnessStandard): LoudnessReport {
    const std = standard ?? this.standard;
    const targetLufs = std === "streaming" ? -14.0 : std === "atsc_a85" ? -24.0 : -23.0;
    const tolerance = std === "streaming" ? 1.5 : 1.0;

    const mMean = this.momentaryCount > 0 ? this.momentarySum / this.momentaryCount : 0;
    const momentaryLufs = mMean > 1e-12 ? Math.max(-70.0, -0.691 + 10 * Math.log10(mMean)) : -70.0;

    const sMean = this.shortTermCount > 0 ? this.shortTermSum / this.shortTermCount : 0;
    const shortTermLufs = sMean > 1e-12 ? Math.max(-70.0, -0.691 + 10 * Math.log10(sMean)) : -70.0;

    let integratedLufs = -70.0;
    if (this.gatedBlocksEnergy.length > 0) {
      const absThresholdLinear = Math.pow(10, (-70 + 0.691) / 10);
      const absPassed = this.gatedBlocksEnergy.filter((z) => z > absThresholdLinear);

      if (absPassed.length > 0) {
        const absMean = absPassed.reduce((a, b) => a + b, 0) / absPassed.length;
        const absLufs = -0.691 + 10 * Math.log10(absMean);

        const relThresholdLinear = Math.pow(10, (absLufs - 10.0 + 0.691) / 10);
        const relPassed = absPassed.filter((z) => z > relThresholdLinear);

        if (relPassed.length > 0) {
          const relMean = relPassed.reduce((a, b) => a + b, 0) / relPassed.length;
          integratedLufs = -0.691 + 10 * Math.log10(relMean);
        }
      }
    }

    let loudnessRangeLu = 0.0;
    if (this.shortTermHistory.length >= 5) {
      const sorted = [...this.shortTermHistory].sort((a, b) => a - b);
      const p10Index = Math.floor(sorted.length * 0.1);
      const p95Index = Math.min(sorted.length - 1, Math.floor(sorted.length * 0.95));
      loudnessRangeLu = Math.max(0, Math.round((sorted[p95Index] - sorted[p10Index]) * 10) / 10);
    }

    const truePeakDbtp = this.maxTruePeakLinear > 1e-12
      ? Math.round(20 * Math.log10(this.maxTruePeakLinear) * 10) / 10
      : -60.0;

    const diff = Math.abs(integratedLufs - targetLufs);
    const isCompliant = integratedLufs > -60 && diff <= tolerance;
    const gainRecommendationDb = integratedLufs > -60
      ? Math.round((targetLufs - integratedLufs) * 10) / 10
      : 0;

    return {
      momentaryLufs: Math.round(momentaryLufs * 10) / 10,
      shortTermLufs: Math.round(shortTermLufs * 10) / 10,
      integratedLufs: Math.round(integratedLufs * 10) / 10,
      loudnessRangeLu,
      truePeakDbtp,
      targetLufs,
      isCompliant,
      gainRecommendationDb,
    };
  }
}

/**
 * AudioClockMaster manages a hardware-synchronized Web Audio context
 * providing monotonic, drift-free master clock timing, stereo panning,
 * mute/solo matrix routing, real-time peak VU metering, and ITU-R BS.1770-4
 * / EBU R128 loudness compliance metering.
 */
class AudioClockMaster {
  private ctx: AudioContext | null = null;
  private masterGain: GainNode | null = null;
  private masterLimiter: DynamicsCompressorNode | null = null;
  private masterAnalyser: AnalyserNode | null = null;
  private scriptProcessor: ScriptProcessorNode | null = null;
  private ebuMeter = new EbuR128WebMeter(48000, "ebu_r128");

  private trackInputs = new Map<number, GainNode>();
  private trackGains = new Map<number, GainNode>();
  private trackPanners = new Map<number, StereoPannerNode>();
  private trackAnalysers = new Map<number, AnalyserNode>();
  private trackDenoiseFilters = new Map<number, BiquadFilterNode>();
  private trackLowShelfs = new Map<number, BiquadFilterNode>();
  private trackMidPeaks = new Map<number, BiquadFilterNode>();
  private trackHighShelfs = new Map<number, BiquadFilterNode>();
  private trackCompressors = new Map<number, DynamicsCompressorNode>();
  private trackPans = new Map<number, number>();
  private trackRawGains = new Map<number, number>();
  private trackMutes = new Map<number, boolean>();
  private trackSolos = new Map<number, boolean>();

  getContext(): AudioContext | null {
    if (typeof window === "undefined") return null;
    if (!this.ctx) {
      const AudioCtx = window.AudioContext || (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
      if (AudioCtx) {
        this.ctx = new AudioCtx();
        this.masterGain = this.ctx.createGain();

        // Master Limiter (True-Peak brickwall at -0.5 dB)
        this.masterLimiter = this.ctx.createDynamicsCompressor();
        this.masterLimiter.threshold.setValueAtTime(-0.5, this.ctx.currentTime);
        this.masterLimiter.knee.setValueAtTime(0, this.ctx.currentTime);
        this.masterLimiter.ratio.setValueAtTime(20, this.ctx.currentTime);
        this.masterLimiter.attack.setValueAtTime(0.001, this.ctx.currentTime);
        this.masterLimiter.release.setValueAtTime(0.05, this.ctx.currentTime);

        this.masterAnalyser = this.ctx.createAnalyser();
        this.masterAnalyser.fftSize = 256;
        this.masterAnalyser.smoothingTimeConstant = 0.5;

        this.masterGain.connect(this.masterLimiter);
        this.masterLimiter.connect(this.masterAnalyser);
        this.masterAnalyser.connect(this.ctx.destination);

        // Real-time EBU R128 Loudness Processing Tap
        try {
          if (this.ctx.createScriptProcessor) {
            this.scriptProcessor = this.ctx.createScriptProcessor(2048, 2, 2);
            this.scriptProcessor.onaudioprocess = (e) => {
              const inL = e.inputBuffer.getChannelData(0);
              const inR = e.inputBuffer.numberOfChannels > 1 ? e.inputBuffer.getChannelData(1) : inL;
              this.ebuMeter.processBuffer(inL, inR);
            };
            this.masterGain.connect(this.scriptProcessor);
            const silenceGain = this.ctx.createGain();
            silenceGain.gain.setValueAtTime(0, this.ctx.currentTime);
            this.scriptProcessor.connect(silenceGain);
            silenceGain.connect(this.ctx.destination);
          }
        } catch {
          // Graceful fallback if ScriptProcessor is restricted in host environment
        }
      }
    }
    if (this.ctx && this.ctx.state === "suspended") {
      void this.ctx.resume().catch(() => {
        // User gesture may be required before resuming audio
      });
    }
    return this.ctx;
  }

  getCurrentTime(): number {
    const ctx = this.getContext();
    return ctx ? ctx.currentTime : performance.now() / 1000;
  }

  getTrackGainNode(trackId: number, gainDb = 0): GainNode | null {
    const ctx = this.getContext();
    if (!ctx || !this.masterGain) return null;
    let inputNode = this.trackInputs.get(trackId);
    if (!inputNode) {
      inputNode = ctx.createGain();
      this.trackInputs.set(trackId, inputNode);

      // 1. Denoise Filter (High-pass rumble filter at 80 Hz)
      const denoise = ctx.createBiquadFilter();
      denoise.type = "highpass";
      denoise.frequency.setValueAtTime(10, ctx.currentTime);
      denoise.Q.setValueAtTime(0.707, ctx.currentTime);
      this.trackDenoiseFilters.set(trackId, denoise);

      // 2. 3-Band Parametric Equalizer
      const lowShelf = ctx.createBiquadFilter();
      lowShelf.type = "lowshelf";
      lowShelf.frequency.setValueAtTime(80, ctx.currentTime);
      lowShelf.gain.setValueAtTime(0, ctx.currentTime);
      this.trackLowShelfs.set(trackId, lowShelf);

      const midPeak = ctx.createBiquadFilter();
      midPeak.type = "peaking";
      midPeak.frequency.setValueAtTime(1000, ctx.currentTime);
      midPeak.Q.setValueAtTime(1.0, ctx.currentTime);
      midPeak.gain.setValueAtTime(0, ctx.currentTime);
      this.trackMidPeaks.set(trackId, midPeak);

      const highShelf = ctx.createBiquadFilter();
      highShelf.type = "highshelf";
      highShelf.frequency.setValueAtTime(8000, ctx.currentTime);
      highShelf.gain.setValueAtTime(0, ctx.currentTime);
      this.trackHighShelfs.set(trackId, highShelf);

      // 3. Dynamics Compressor
      const compressor = ctx.createDynamicsCompressor();
      compressor.threshold.setValueAtTime(0, ctx.currentTime);
      compressor.ratio.setValueAtTime(1, ctx.currentTime);
      compressor.attack.setValueAtTime(0.005, ctx.currentTime);
      compressor.release.setValueAtTime(0.1, ctx.currentTime);
      this.trackCompressors.set(trackId, compressor);

      // 4. Track Output Gain
      const gainNode = ctx.createGain();
      this.trackGains.set(trackId, gainNode);

      // 5. Stereo Panner
      let pannerNode: StereoPannerNode | null = null;
      if (ctx.createStereoPanner) {
        pannerNode = ctx.createStereoPanner();
        this.trackPanners.set(trackId, pannerNode);
      }

      // 6. Track Analyser
      const analyser = ctx.createAnalyser();
      analyser.fftSize = 256;
      analyser.smoothingTimeConstant = 0.4;
      this.trackAnalysers.set(trackId, analyser);

      // Connect DSP chain: input -> denoise -> lowShelf -> midPeak -> highShelf -> compressor -> gainNode -> panner -> analyser -> masterGain
      inputNode.connect(denoise);
      denoise.connect(lowShelf);
      lowShelf.connect(midPeak);
      midPeak.connect(highShelf);
      highShelf.connect(compressor);
      compressor.connect(gainNode);

      if (pannerNode) {
        gainNode.connect(pannerNode);
        pannerNode.connect(analyser);
      } else {
        gainNode.connect(analyser);
      }
      analyser.connect(this.masterGain);
    }
    const gainNode = this.trackGains.get(trackId);
    if (gainNode) {
      const linearGain = Math.max(0, Math.min(2, 10 ** (gainDb / 20)));
      gainNode.gain.setValueAtTime(linearGain, ctx.currentTime);
    }
    return inputNode;
  }

  updateTrackGain(trackId: number, gainDb: number): void {
    const ctx = this.getContext();
    this.trackRawGains.set(trackId, gainDb);
    const node = this.trackGains.get(trackId);
    if (ctx && node) {
      const isMuted = this.trackMutes.get(trackId) ?? false;
      const anySolo = Array.from(this.trackSolos.values()).some(Boolean);
      const isSolo = this.trackSolos.get(trackId) ?? false;
      const isAudible = !isMuted && (!anySolo || isSolo);
      const linearGain = isAudible ? Math.max(0, Math.min(2, 10 ** (gainDb / 20))) : 0;
      node.gain.setTargetAtTime(linearGain, ctx.currentTime, 0.02);
    }
  }

  updateTrackPan(trackId: number, pan: number): void {
    const ctx = this.getContext();
    this.trackPans.set(trackId, pan);
    const panner = this.trackPanners.get(trackId);
    if (ctx && panner && panner.pan) {
      panner.pan.setTargetAtTime(Math.max(-1, Math.min(1, pan)), ctx.currentTime, 0.02);
    }
  }

  updateTrackAudioProcessing(trackId: number, proc: TrackAudioProcessing | undefined): void {
    const ctx = this.getContext();
    if (!ctx) return;
    this.getTrackGainNode(trackId);

    const denoise = this.trackDenoiseFilters.get(trackId);
    if (denoise) {
      const freq = proc?.denoise_enabled ? 80 : 10;
      denoise.frequency.setTargetAtTime(freq, ctx.currentTime, 0.02);
    }

    const lowShelf = this.trackLowShelfs.get(trackId);
    if (lowShelf) {
      const g = proc?.eq_enabled ? (proc.eq_low_gain ?? 0) : 0;
      lowShelf.gain.setTargetAtTime(g, ctx.currentTime, 0.02);
    }

    const midPeak = this.trackMidPeaks.get(trackId);
    if (midPeak) {
      const g = proc?.eq_enabled ? (proc.eq_mid_gain ?? 0) : 0;
      midPeak.gain.setTargetAtTime(g, ctx.currentTime, 0.02);
    }

    const highShelf = this.trackHighShelfs.get(trackId);
    if (highShelf) {
      const g = proc?.eq_enabled ? (proc.eq_high_gain ?? 0) : 0;
      highShelf.gain.setTargetAtTime(g, ctx.currentTime, 0.02);
    }

    const comp = this.trackCompressors.get(trackId);
    if (comp) {
      if (proc?.comp_enabled) {
        comp.threshold.setTargetAtTime(proc.comp_threshold ?? -24, ctx.currentTime, 0.02);
        comp.ratio.setTargetAtTime(proc.comp_ratio ?? 4, ctx.currentTime, 0.02);
        comp.attack.setTargetAtTime(proc.comp_attack ?? 0.005, ctx.currentTime, 0.02);
        comp.release.setTargetAtTime(proc.comp_release ?? 0.1, ctx.currentTime, 0.02);
      } else {
        comp.threshold.setTargetAtTime(0, ctx.currentTime, 0.02);
        comp.ratio.setTargetAtTime(1, ctx.currentTime, 0.02);
      }
    }
  }

  getTrackGainReduction(trackId: number): number {
    const comp = this.trackCompressors.get(trackId);
    return comp ? comp.reduction : 0;
  }

  getMasterGainReduction(): number {
    return this.masterLimiter ? this.masterLimiter.reduction : 0;
  }

  updateTracksState(tracks: Track[]): void {
    const ctx = this.getContext();
    if (!ctx) return;
    const anySolo = tracks.some((t) => t.solo);
    for (const track of tracks) {
      this.trackRawGains.set(track.id, track.gain_db);
      this.trackMutes.set(track.id, !!track.muted);
      this.trackSolos.set(track.id, !!track.solo);
      this.trackPans.set(track.id, track.pan ?? 0);
      this.updateTrackAudioProcessing(track.id, track.audio_processing);

      const panner = this.trackPanners.get(track.id);
      if (panner && panner.pan) {
        panner.pan.setTargetAtTime(Math.max(-1, Math.min(1, track.pan ?? 0)), ctx.currentTime, 0.02);
      }

      const isAudible = !track.muted && (!anySolo || track.solo);
      const linearGain = isAudible ? Math.max(0, Math.min(2, 10 ** (track.gain_db / 20))) : 0;
      const gainNode = this.trackGains.get(track.id);
      if (gainNode) {
        gainNode.gain.setTargetAtTime(linearGain, ctx.currentTime, 0.02);
      }
    }
  }

  setMasterGain(gainDb: number): void {
    const ctx = this.getContext();
    if (ctx && this.masterGain) {
      const linearGain = Math.max(0, Math.min(2, 10 ** (gainDb / 20)));
      this.masterGain.gain.setTargetAtTime(linearGain, ctx.currentTime, 0.02);
    }
  }

  getTrackLevels(trackId: number): { left: number; right: number; peakDb: number } {
    const analyser = this.trackAnalysers.get(trackId);
    if (!analyser) return { left: 0, right: 0, peakDb: -60 };
    const buffer = new Uint8Array(analyser.frequencyBinCount);
    analyser.getByteTimeDomainData(buffer);
    let maxDelta = 0;
    for (let i = 0; i < buffer.length; i++) {
      const v = Math.abs(buffer[i] - 128) / 128;
      if (v > maxDelta) maxDelta = v;
    }
    const peakDb = maxDelta > 0.001 ? 20 * Math.log10(maxDelta) : -60;
    const pan = this.trackPans.get(trackId) ?? 0;
    const leftMult = pan <= 0 ? 1 : Math.max(0, 1 - pan);
    const rightMult = pan >= 0 ? 1 : Math.max(0, 1 + pan);
    return {
      left: Math.min(1, maxDelta * leftMult),
      right: Math.min(1, maxDelta * rightMult),
      peakDb: Math.max(-60, Math.min(6, peakDb)),
    };
  }

  getMasterLevels(): { left: number; right: number; peakDb: number } {
    if (!this.masterAnalyser) return { left: 0, right: 0, peakDb: -60 };
    const buffer = new Uint8Array(this.masterAnalyser.frequencyBinCount);
    this.masterAnalyser.getByteTimeDomainData(buffer);
    let maxDelta = 0;
    for (let i = 0; i < buffer.length; i++) {
      const v = Math.abs(buffer[i] - 128) / 128;
      if (v > maxDelta) maxDelta = v;
    }
    const peakDb = maxDelta > 0.001 ? 20 * Math.log10(maxDelta) : -60;
    return {
      left: Math.min(1, maxDelta),
      right: Math.min(1, maxDelta),
      peakDb: Math.max(-60, Math.min(6, peakDb)),
    };
  }

  getLoudnessReport(standard?: LoudnessStandard): LoudnessReport {
    return this.ebuMeter.getReport(standard);
  }

  resetLoudness(): void {
    this.ebuMeter.reset();
  }

  setLoudnessStandard(standard: LoudnessStandard): void {
    this.ebuMeter.setStandard(standard);
  }

  applyLoudnessAutoTrim(standard?: LoudnessStandard): number {
    const report = this.getLoudnessReport(standard);
    const delta = report.gainRecommendationDb;
    if (Math.abs(delta) > 0.05) {
      const currentMaster = this.masterGain ? Math.round(20 * Math.log10(this.masterGain.gain.value) * 10) / 10 : 0;
      const targetGainDb = Math.max(-60, Math.min(12, currentMaster + delta));
      this.setMasterGain(targetGainDb);
      return targetGainDb;
    }
    return 0;
  }

  smoothMicroFade(gainNode: GainNode, fadeIn: boolean, durationSec = 0.015): void {
    const ctx = this.getContext();
    if (!ctx) return;
    const now = ctx.currentTime;
    if (fadeIn) {
      gainNode.gain.setValueAtTime(0.001, now);
      gainNode.gain.exponentialRampToValueAtTime(1.0, now + durationSec);
    } else {
      gainNode.gain.setValueAtTime(1.0, now);
      gainNode.gain.exponentialRampToValueAtTime(0.001, now + durationSec);
    }
  }
}

export const audioClockMaster = new AudioClockMaster();

/**
 * Evaluates a cubic bezier curve at parameter t in [0, 1] with control points (0,0), (x1, y1), (x2, y2), (1,1).
 */
export function solveCubicBezier(t: number, x1: number, y1: number, x2: number, y2: number): number {
  const clampedT = Math.max(0, Math.min(1, t));
  if (clampedT <= 0) return 0;
  if (clampedT >= 1) return 1;

  // B(s) = 3(1-s)^2*s*P1 + 3(1-s)*s^2*P2 + s^3
  const cx = 3 * x1;
  const bx = 3 * (x2 - x1) - cx;
  const ax = 1 - cx - bx;

  const cy = 3 * y1;
  const by = 3 * (y2 - y1) - cy;
  const ay = 1 - cy - by;

  const sampleCurveX = (s: number) => ((ax * s + bx) * s + cx) * s;
  const sampleCurveY = (s: number) => ((ay * s + by) * s + cy) * s;
  const sampleCurveDerivativeX = (s: number) => (3 * ax * s + 2 * bx) * s + cx;

  // Newton-Raphson iteration
  let s = clampedT;
  for (let i = 0; i < 8; i++) {
    const x = sampleCurveX(s) - clampedT;
    if (Math.abs(x) < 1e-5) return sampleCurveY(s);
    const d = sampleCurveDerivativeX(s);
    if (Math.abs(d) < 1e-5) break;
    s -= x / d;
  }

  // Binary subdivision fallback
  let lo = 0;
  let hi = 1;
  s = clampedT;
  for (let i = 0; i < 12; i++) {
    const x = sampleCurveX(s);
    if (Math.abs(x - clampedT) < 1e-4) return sampleCurveY(s);
    if (clampedT > x) lo = s;
    else hi = s;
    s = (hi + lo) / 2;
  }

  return sampleCurveY(s);
}

/**
 * Extracts active subtitle item at a specific timeline playhead in ticks.
 */
export function getActiveSubtitle(subtitles: SubtitleItem[] | undefined, playheadTicks: number): SubtitleItem | null {
  if (!subtitles || subtitles.length === 0) return null;
  for (const sub of subtitles) {
    if (playheadTicks >= sub.start_ticks && playheadTicks < sub.end_ticks) {
      return sub;
    }
  }
  return null;
}

/**
 * Computes an animated keyframe property value with bezier & ease curve interpolation.
 */
export function interpolateKeyframe(
  clip: Clip | undefined,
  property: KeyframeProperty,
  clipTimeTicks: number,
  fallback: number
): number {
  if (!clip || !clip.keyframes) return fallback;
  const values = clip.keyframes.filter((k) => k.property === property).sort((a, b) => a.time - b.time);
  if (!values.length) return fallback;
  if (clipTimeTicks <= values[0].time) return values[0].value;
  const last = values[values.length - 1];
  if (clipTimeTicks >= last.time) return last.value;
  const nextIndex = values.findIndex((k) => k.time >= clipTimeTicks);
  const prev = values[nextIndex - 1];
  const next = values[nextIndex];
  if (!prev || !next) return fallback;
  const span = next.time - prev.time;
  if (span <= 0) return prev.value;
  const ratio = Math.max(0, Math.min(1, (clipTimeTicks - prev.time) / span));

  const interp = prev.interpolation ?? "linear";
  let progress = ratio;

  switch (interp) {
    case "linear":
      progress = ratio;
      break;
    case "ease_in":
      progress = solveCubicBezier(ratio, 0.42, 0.0, 1.0, 1.0);
      break;
    case "ease_out":
      progress = solveCubicBezier(ratio, 0.0, 0.0, 0.58, 1.0);
      break;
    case "ease_in_out":
      progress = solveCubicBezier(ratio, 0.42, 0.0, 0.58, 1.0);
      break;
    case "bezier": {
      const x1 = prev.handle_out_x ?? 0.33;
      const y1 = prev.handle_out_y ?? 0.0;
      const x2 = next.handle_in_x ?? 0.67;
      const y2 = next.handle_in_y ?? 1.0;
      progress = solveCubicBezier(ratio, x1, y1, x2, y2);
      break;
    }
    default:
      progress = ratio;
      break;
  }

  return prev.value + (next.value - prev.value) * progress;
}

export interface TransitionBlend {
  active: boolean;
  kind: TransitionKind;
  progress: number; // 0.0 to 1.0
  deckAStyle: CSSProperties;
  deckBStyle: CSSProperties;
}

/**
 * Calculates real-time visual transition styling for Dual-Deck video playback.
 */
export function computeTransitionBlend(
  transition: Transition | null,
  playhead: number
): TransitionBlend {
  if (!transition) {
    return {
      active: false,
      kind: "crossfade",
      progress: 0,
      deckAStyle: { opacity: 1 },
      deckBStyle: { opacity: 0, pointerEvents: "none" },
    };
  }

  const halfDur = Math.floor(transition.duration / 2);
  const start = transition.cut_at - halfDur;
  const end = transition.cut_at + halfDur;

  if (playhead < start || playhead > end) {
    return {
      active: false,
      kind: transition.kind,
      progress: playhead < start ? 0 : 1,
      deckAStyle: { opacity: playhead < start ? 1 : 0 },
      deckBStyle: { opacity: playhead >= end ? 1 : 0 },
    };
  }

  const duration = Math.max(1, end - start);
  const rawProgress = (playhead - start) / duration;
  const p = Math.max(0, Math.min(1, rawProgress));

  switch (transition.kind) {
    case "crossfade":
      return {
        active: true,
        kind: transition.kind,
        progress: p,
        deckAStyle: { opacity: 1 - p },
        deckBStyle: { opacity: p },
      };

    case "dip_to_black":
      if (p < 0.5) {
        return {
          active: true,
          kind: transition.kind,
          progress: p,
          deckAStyle: { opacity: Math.max(0, 1 - p * 2) },
          deckBStyle: { opacity: 0 },
        };
      }
      return {
        active: true,
        kind: transition.kind,
        progress: p,
        deckAStyle: { opacity: 0 },
        deckBStyle: { opacity: Math.max(0, (p - 0.5) * 2) },
      };

    case "dip_to_white": {
      const flash = Math.sin(p * Math.PI) * 2.5;
      return {
        active: true,
        kind: transition.kind,
        progress: p,
        deckAStyle: {
          opacity: 1 - p,
          filter: `brightness(${1 + flash})`,
        },
        deckBStyle: {
          opacity: p,
          filter: `brightness(${1 + flash})`,
        },
      };
    }

    case "wipe_left":
      return {
        active: true,
        kind: transition.kind,
        progress: p,
        deckAStyle: { opacity: 1 },
        deckBStyle: {
          opacity: 1,
          clipPath: `inset(0 0 0 ${(1 - p) * 100}%)`,
        },
      };

    case "wipe_right":
      return {
        active: true,
        kind: transition.kind,
        progress: p,
        deckAStyle: { opacity: 1 },
        deckBStyle: {
          opacity: 1,
          clipPath: `inset(0 ${(1 - p) * 100}% 0 0)`,
        },
      };

    case "slide_left":
      return {
        active: true,
        kind: transition.kind,
        progress: p,
        deckAStyle: {
          transform: `translateX(${-p * 100}%)`,
        },
        deckBStyle: {
          transform: `translateX(${(1 - p) * 100}%)`,
        },
      };

    case "slide_right":
      return {
        active: true,
        kind: transition.kind,
        progress: p,
        deckAStyle: {
          transform: `translateX(${p * 100}%)`,
        },
        deckBStyle: {
          transform: `translateX(${-(1 - p) * 100}%)`,
        },
      };

    default:
      return {
        active: true,
        kind: "crossfade",
        progress: p,
        deckAStyle: { opacity: 1 - p },
        deckBStyle: { opacity: p },
      };
  }
}

export interface DeckConfig {
  mediaId: number | null;
  src: string;
  currentTime: number;
  visible: boolean;
  style: CSSProperties;
  vignette?: { intensity: number; radius: number; softness: number } | null;
  chromaKey?: ChromaKeyParams | null;
  clip: Clip | null;
  track: Track | null;
  isPreload: boolean;
}

export interface ActiveAudioClip {
  track: Track;
  clip: Clip;
  media: MediaAsset;
  src: string;
  currentTime: number;
  gainDb: number;
}

export interface TimelinePlaybackSnapshot {
  playhead: number;
  sequenceDuration: number;
  deckA: DeckConfig;
  deckB: DeckConfig;
  activeDeck: "A" | "B";
  inTransition: boolean;
  transitionKind: TransitionKind | null;
  activeAudioClips: ActiveAudioClip[];
  activeTitleClip: { track: Track; clip: Clip; media: MediaAsset; title: TitleProperties } | null;
  activeSubtitle: SubtitleItem | null;
  hasVideoMedia: boolean;
}

/**
 * Resolves the URL for a media asset, preferring prepared WebM/VP8 proxies for video.
 */
export function resolveMediaStreamUrl(
  mediaAsset: MediaAsset | null | undefined,
  preparedPreviews: Record<number, string>,
  mediaServerPort: number,
  mediaServerToken?: string
): string {
  if (!mediaAsset || mediaServerPort <= 0) return "";
  const proxyPath = preparedPreviews[mediaAsset.id];
  const targetPath = proxyPath || mediaAsset.path;
  const tokenPart = mediaServerToken ? `&token=${encodeURIComponent(mediaServerToken)}` : "";
  return `http://127.0.0.1:${mediaServerPort}/media?path=${encodeURIComponent(targetPath)}${tokenPart}`;
}

/**
 * Builds the complete dual-deck video and multi-track audio playback evaluation
 * for any given timeline playhead position.
 */
export function evaluateTimelinePlayback(
  project: ProjectDocument | null,
  playhead: number,
  mediaList: MediaAsset[],
  preparedPreviews: Record<number, string>,
  mediaServerPort: number,
  mediaServerToken?: string
): TimelinePlaybackSnapshot {
  const emptyResult: TimelinePlaybackSnapshot = {
    playhead,
    sequenceDuration: 0,
    deckA: { mediaId: null, src: "", currentTime: 0, visible: false, style: {}, clip: null, track: null, isPreload: false },
    deckB: { mediaId: null, src: "", currentTime: 0, visible: false, style: {}, clip: null, track: null, isPreload: false },
    activeDeck: "A",
    inTransition: false,
    transitionKind: null,
    activeAudioClips: [],
    activeTitleClip: null,
    activeSubtitle: null,
    hasVideoMedia: false,
  };

  if (!project) return emptyResult;

  const tracks = project.project.sequence.tracks;
  const projectWidth = project.project.settings.width || 1920;
  const projectHeight = project.project.settings.height || 1080;
  const subtitles = project.project.sequence.subtitles;
  const activeSubtitle = getActiveSubtitle(subtitles, playhead);

  // Compute sequence duration across all tracks
  let maxEnd = 0;
  for (const track of tracks) {
    for (const clip of track.clips) {
      const end = clip.timeline_start + clip.duration;
      if (end > maxEnd) maxEnd = end;
    }
  }
  const sequenceDuration = maxEnd;

  // 1. Gather all active audio clips across audio and video tracks (including compound sequences)
  const activeAudioClips: ActiveAudioClip[] = [];
  for (const track of tracks) {
    for (const clip of track.clips) {
      if (playhead >= clip.timeline_start && playhead < clip.timeline_start + clip.duration) {
        const item = mediaList.find((m) => m.id === clip.media_id);
        if (item && (item.kind === "compound_sequence" || clip.compound_sequence_id != null)) {
          const nestedSeq = item.nested_sequence || project.project.sequences?.find((s) => s.id === clip.compound_sequence_id);
          if (nestedSeq) {
            const speed = clip.speed ?? 1.0;
            const localPlayhead = Math.max(0, clip.source_in + Math.round((playhead - clip.timeline_start) * speed));
            for (const nTrack of nestedSeq.tracks) {
              for (const nClip of nTrack.clips) {
                if (localPlayhead >= nClip.timeline_start && localPlayhead < nClip.timeline_start + nClip.duration) {
                  const nItem = mediaList.find((m) => m.id === nClip.media_id);
                  if (nItem && (nItem.kind === "audio" || (nItem.kind === "video" && nTrack.kind === "video"))) {
                    const src = resolveMediaStreamUrl(nItem, preparedPreviews, mediaServerPort, mediaServerToken);
                    if (src) {
                      const currentTime = Math.max(0, (nClip.source_in + (localPlayhead - nClip.timeline_start) * (nClip.speed ?? 1.0)) / TICKS_PER_SECOND);
                      activeAudioClips.push({
                        track: nTrack,
                        clip: nClip,
                        media: nItem,
                        src,
                        currentTime,
                        gainDb: (track.gain_db ?? 0) + (nTrack.gain_db ?? 0),
                      });
                    }
                  }
                }
              }
            }
          }
        } else if (item && (item.kind === "audio" || (item.kind === "video" && track.kind === "video"))) {
          const src = resolveMediaStreamUrl(item, preparedPreviews, mediaServerPort, mediaServerToken);
          if (src) {
            const speed = clip.speed ?? 1.0;
            const currentTime = Math.max(0, (clip.source_in + (playhead - clip.timeline_start) * speed) / TICKS_PER_SECOND);
            activeAudioClips.push({
              track,
              clip,
              media: item,
              src,
              currentTime,
              gainDb: track.gain_db ?? 0,
            });
          }
        }
      }
    }
  }

  // 2. Identify active title overlay
  let activeTitleClip: { track: Track; clip: Clip; media: MediaAsset; title: TitleProperties } | null = null;
  for (const track of tracks) {
    if (track.kind !== "video") continue;
    for (const clip of track.clips) {
      if (playhead >= clip.timeline_start && playhead < clip.timeline_start + clip.duration) {
        const item = mediaList.find((m) => m.id === clip.media_id);
        if (item?.kind === "title" && item.title) {
          activeTitleClip = { track, clip, media: item, title: item.title };
          break;
        } else if (item && (item.kind === "compound_sequence" || clip.compound_sequence_id != null)) {
          const nestedSeq = item.nested_sequence || project.project.sequences?.find((s) => s.id === clip.compound_sequence_id);
          if (nestedSeq) {
            const speed = clip.speed ?? 1.0;
            const localPlayhead = Math.max(0, clip.source_in + Math.round((playhead - clip.timeline_start) * speed));
            for (const nTrack of nestedSeq.tracks) {
              if (nTrack.kind !== "video") continue;
              for (const nClip of nTrack.clips) {
                if (localPlayhead >= nClip.timeline_start && localPlayhead < nClip.timeline_start + nClip.duration) {
                  const nItem = mediaList.find((m) => m.id === nClip.media_id);
                  if (nItem?.kind === "title" && nItem.title) {
                    activeTitleClip = { track: nTrack, clip: nClip, media: nItem, title: nItem.title };
                    break;
                  }
                }
              }
              if (activeTitleClip) break;
            }
          }
        }
      }
    }
    if (activeTitleClip) break;
  }

  // 3. Find primary video track and visual clips (including compound sequences)
  const videoTracks = tracks.filter((t) => t.kind === "video");
  let activeTrack: Track | null = null;
  let activeClip: Clip | null = null;
  let activeMedia: MediaAsset | null = null;

  for (const track of videoTracks) {
    for (const clip of track.clips) {
      if (playhead >= clip.timeline_start && playhead < clip.timeline_start + clip.duration) {
        const item = mediaList.find((m) => m.id === clip.media_id);
        if (item && (item.kind === "video" || item.kind === "image")) {
          activeTrack = track;
          activeClip = clip;
          activeMedia = item;
          break;
        } else if (item && (item.kind === "compound_sequence" || clip.compound_sequence_id != null)) {
          const nestedSeq = item.nested_sequence || project.project.sequences?.find((s) => s.id === clip.compound_sequence_id);
          if (nestedSeq) {
            const speed = clip.speed ?? 1.0;
            const localPlayhead = Math.max(0, clip.source_in + Math.round((playhead - clip.timeline_start) * speed));
            for (const nTrack of nestedSeq.tracks.filter((t) => t.kind === "video")) {
              for (const nClip of nTrack.clips) {
                if (localPlayhead >= nClip.timeline_start && localPlayhead < nClip.timeline_start + nClip.duration) {
                  const nItem = mediaList.find((m) => m.id === nClip.media_id);
                  if (nItem && (nItem.kind === "video" || nItem.kind === "image")) {
                    activeTrack = track;
                    activeClip = {
                      ...nClip,
                      timeline_start: clip.timeline_start,
                      duration: clip.duration,
                      source_in: Math.max(0, nClip.source_in + (localPlayhead - nClip.timeline_start)),
                    };
                    activeMedia = nItem;
                    break;
                  }
                }
              }
              if (activeClip) break;
            }
          }
        }
      }
    }
    if (activeClip) break;
  }

  // 4. Check for active transition
  let activeTransition: Transition | null = null;
  let transitionTrack: Track | null = null;

  for (const track of videoTracks) {
    if (!track.transitions) continue;
    for (const trans of track.transitions) {
      const halfDur = Math.floor(trans.duration / 2);
      if (playhead >= trans.cut_at - halfDur && playhead <= trans.cut_at + halfDur) {
        activeTransition = trans;
        transitionTrack = track;
        break;
      }
    }
    if (activeTransition) break;
  }

  // Helper to extract vignette params from clip
  const getClipVignette = (clip: Clip | null): { intensity: number; radius: number; softness: number } | null => {
    if (!clip || !clip.effects) return null;
    const vig = clip.effects.find((e) => e.enabled && e.kind === "vignette");
    if (!vig) return null;
    const p = vig.params as VignetteParams;
    return {
      intensity: p.intensity ?? 0.5,
      radius: p.radius ?? 0.8,
      softness: p.softness ?? 0.5,
    };
  };

  // Helper to extract chroma key params from clip
  const getClipChromaKey = (clip: Clip | null): ChromaKeyParams | null => {
    if (!clip || !clip.effects) return null;
    const chroma = clip.effects.find((e) => e.enabled && e.kind === "chroma_key");
    if (!chroma) return null;
    return chroma.params as ChromaKeyParams;
  };

  // Visual transform and composable effect stack styling helper
  const computeVisualStyles = (clip: Clip | null, isPrimary = true): CSSProperties => {
    if (!clip) return {};
    const clipTime = playhead - clip.timeline_start;

    let brightness = 1;
    let contrast = 1;
    let saturate = 1;
    let hueRotate = 0;
    let blurPx = 0;
    let cropInset: string | null = null;

    if (clip.effects && clip.effects.length > 0) {
      for (const eff of clip.effects) {
        if (!eff.enabled) continue;
        if (eff.kind === "color_correction") {
          const p = eff.params as ColorCorrectionParams;
          brightness *= 2 ** (p.exposure ?? 0);
          contrast *= p.contrast ?? 1;
          saturate *= p.saturation ?? 1;
          hueRotate += (p.hue ?? 0) * 180;
        } else if (eff.kind === "gaussian_blur") {
          const p = eff.params as GaussianBlurParams;
          blurPx += (p.sigma ?? 0) * 2;
        } else if (eff.kind === "crop") {
          const p = eff.params as CropParams;
          cropInset = `inset(${p.top ?? 0}% ${p.right ?? 0}% ${p.bottom ?? 0}% ${p.left ?? 0}%)`;
        } else if (eff.kind === "lut") {
          const p = eff.params as LutParams;
          const blend = Math.max(0, Math.min(1, p.intensity ?? 1.0));
          const preset = p.preset_id ?? "teal_orange";
          if (blend > 0) {
            switch (preset) {
              case "teal_orange":
                contrast *= 1 + 0.15 * blend;
                saturate *= 1 + 0.25 * blend;
                hueRotate += -12 * blend;
                break;
              case "vintage":
                contrast *= 1 - 0.08 * blend;
                brightness *= 1 + 0.05 * blend;
                saturate *= 1 - 0.15 * blend;
                hueRotate += 8 * blend;
                break;
              case "film_noir":
                saturate *= 1 - 1.0 * blend;
                contrast *= 1 + 0.35 * blend;
                break;
              case "rec709":
                contrast *= 1 + 0.12 * blend;
                saturate *= 1 + 0.12 * blend;
                break;
              default:
                contrast *= 1 + 0.1 * blend;
                saturate *= 1 + 0.1 * blend;
                break;
            }
          }
        }
      }
    } else {
      const color = clip.color || { exposure: 0, contrast: 1, saturation: 1 };
      brightness = 2 ** (color.exposure ?? 0);
      contrast = color.contrast ?? 1;
      saturate = color.saturation ?? 1;
      hueRotate += (color.hue ?? 0) * 180;
    }

    // Color Wheels (Lift, Gamma, Gain, Offset)
    const wheels = clip.color?.wheels;
    if (wheels) {
      const liftY = wheels.lift?.luma ?? 0;
      const gammaY = wheels.gamma?.luma ?? 0;
      const gainY = wheels.gain?.luma ?? 0;
      const offsetY = wheels.offset?.luma ?? 0;

      brightness *= 1 + (liftY * 0.25 + gainY * 0.35 + offsetY * 0.3);
      contrast *= 1 + (gainY * 0.2 - liftY * 0.15 + gammaY * 0.1);

      const rShift = (wheels.lift?.red ?? 0) * 0.15 + (wheels.gain?.red ?? 0) * 0.2 + (wheels.offset?.red ?? 0) * 0.1;
      const bShift = (wheels.lift?.blue ?? 0) * 0.15 + (wheels.gain?.blue ?? 0) * 0.2 + (wheels.offset?.blue ?? 0) * 0.1;
      const tintDeg = (rShift - bShift) * 40;
      hueRotate += tintDeg;
    }

    // Motion Blur (dynamic shutter angle simulation)
    if (clip.motion_blur?.enabled && (clip.motion_blur.shutter_angle_deg ?? 180) > 0) {
      const shutter = clip.motion_blur.shutter_angle_deg ?? 180;
      const motionBlurPx = (shutter / 360) * 1.5;
      if (motionBlurPx > 0.1) {
        blurPx += motionBlurPx;
      }
    }

    // Fusion Node Graph roto-spline mask polygon
    if (clip.node_graph?.masks && clip.node_graph.masks.length > 0) {
      const activeMask = clip.node_graph.masks[0];
      if (activeMask.points && activeMask.points.length >= 3) {
        const polyCoords = activeMask.points.map((p) => `${p.x}% ${p.y}%`).join(", ");
        cropInset = `polygon(${polyCoords})`;
      }
    }

    const filterParts: string[] = [];
    if (Math.abs(brightness - 1) > 0.001) filterParts.push(`brightness(${brightness.toFixed(3)})`);
    if (Math.abs(contrast - 1) > 0.001) filterParts.push(`contrast(${contrast.toFixed(3)})`);
    if (Math.abs(saturate - 1) > 0.001) filterParts.push(`saturate(${saturate.toFixed(3)})`);
    if (Math.abs(hueRotate) > 0.1) filterParts.push(`hue-rotate(${hueRotate.toFixed(1)}deg)`);
    if (blurPx > 0) filterParts.push(`blur(${blurPx.toFixed(1)}px)`);

    let stabX = 0;
    let stabY = 0;
    if (clip.tracking?.points && clip.tracking.points.length > 0) {
      if (clip.tracking.mode === "stabilize") {
        const pts = clip.tracking.points;
        const smoothness = clip.tracking.smoothness ?? 0.5;
        const pt = pts.find((p) => Math.abs(p.time - clipTime) < TICKS_PER_SECOND / 10) || pts[0];
        if (pt) {
          stabX = (0.5 - pt.x) * projectWidth * smoothness;
          stabY = (0.5 - pt.y) * projectHeight * smoothness;
        }
      }
    }

    const posX = interpolateKeyframe(clip, "position_x", clipTime, 0) + stabX;
    const posY = interpolateKeyframe(clip, "position_y", clipTime, 0) + stabY;
    const scale = interpolateKeyframe(clip, "scale", clipTime, 1);
    const rotation = interpolateKeyframe(clip, "rotation", clipTime, 0);
    const opacity = interpolateKeyframe(clip, "opacity", clipTime, 1);

    const transform = `translate(${(posX / projectWidth) * 100}%, ${(posY / projectHeight) * 100}%) scale(${scale}) rotate(${rotation}deg)`;

    return {
      filter: filterParts.length > 0 ? filterParts.join(" ") : undefined,
      clipPath: cropInset ?? undefined,
      transform: isPrimary ? transform : undefined,
      opacity,
      transformOrigin: "center",
    };
  };

  // Case A: Inside an active Transition boundary
  if (activeTransition && transitionTrack) {
    // Locate the outgoing clip (ending at cut_at) and incoming clip (starting at cut_at)
    const outgoing = transitionTrack.clips.find(
      (c) => Math.abs(c.timeline_start + c.duration - activeTransition!.cut_at) < TICKS_PER_SECOND / 10
    );
    const incoming = transitionTrack.clips.find(
      (c) => Math.abs(c.timeline_start - activeTransition!.cut_at) < TICKS_PER_SECOND / 10
    );

    const outMedia = outgoing ? mediaList.find((m) => m.id === outgoing.media_id) : null;
    const inMedia = incoming ? mediaList.find((m) => m.id === incoming.media_id) : null;

    const outSrc = resolveMediaStreamUrl(outMedia, preparedPreviews, mediaServerPort, mediaServerToken);
    const inSrc = resolveMediaStreamUrl(inMedia, preparedPreviews, mediaServerPort, mediaServerToken);

    const outSpeed = outgoing?.speed ?? 1.0;
    const inSpeed = incoming?.speed ?? 1.0;
    const outTime = outgoing ? Math.max(0, (outgoing.source_in + (playhead - outgoing.timeline_start) * outSpeed) / TICKS_PER_SECOND) : 0;
    const inTime = incoming ? Math.max(0, (incoming.source_in + (playhead - incoming.timeline_start) * inSpeed) / TICKS_PER_SECOND) : 0;

    const blend = computeTransitionBlend(activeTransition, playhead);
    const outBaseStyle = computeVisualStyles(outgoing ?? null);
    const inBaseStyle = computeVisualStyles(incoming ?? null);

    return {
      playhead,
      sequenceDuration,
      deckA: {
        mediaId: outMedia?.id ?? null,
        src: outSrc,
        currentTime: outTime,
        visible: Boolean(outSrc),
        style: { ...outBaseStyle, ...blend.deckAStyle },
        vignette: getClipVignette(outgoing ?? null),
        chromaKey: getClipChromaKey(outgoing ?? null),
        clip: outgoing ?? null,
        track: transitionTrack,
        isPreload: false,
      },
      deckB: {
        mediaId: inMedia?.id ?? null,
        src: inSrc,
        currentTime: inTime,
        visible: Boolean(inSrc),
        style: { ...inBaseStyle, ...blend.deckBStyle },
        vignette: getClipVignette(incoming ?? null),
        chromaKey: getClipChromaKey(incoming ?? null),
        clip: incoming ?? null,
        track: transitionTrack,
        isPreload: false,
      },
      activeDeck: blend.progress < 0.5 ? "A" : "B",
      inTransition: true,
      transitionKind: activeTransition.kind,
      activeAudioClips,
      activeTitleClip,
      activeSubtitle,
      hasVideoMedia: Boolean(outSrc || inSrc),
    };
  }

  // Case B: Standard Single Active Clip with Next Clip Preloaded on Staging Deck
  if (activeClip && activeMedia && activeTrack) {
    const activeSrc = resolveMediaStreamUrl(activeMedia, preparedPreviews, mediaServerPort, mediaServerToken);
    const clipSpeed = activeClip.speed ?? 1.0;
    const activeTime = Math.max(0, (activeClip.source_in + (playhead - activeClip.timeline_start) * clipSpeed) / TICKS_PER_SECOND);
    const activeStyle = computeVisualStyles(activeClip);

    // Look ahead for the next clip on the same video track
    const nextClip = activeTrack.clips.find((c) => c.timeline_start >= activeClip!.timeline_start + activeClip!.duration);
    const nextMedia = nextClip ? mediaList.find((m) => m.id === nextClip.media_id) : null;
    const nextSrc = resolveMediaStreamUrl(nextMedia, preparedPreviews, mediaServerPort, mediaServerToken);
    const nextTime = nextClip ? Math.max(0, nextClip.source_in / TICKS_PER_SECOND) : 0;

    // Deck A is active with current clip, Deck B stages next clip for seamless cut handoff
    return {
      playhead,
      sequenceDuration,
      deckA: {
        mediaId: activeMedia.id,
        src: activeSrc,
        currentTime: activeTime,
        visible: true,
        style: { ...activeStyle, opacity: 1 },
        vignette: getClipVignette(activeClip),
        chromaKey: getClipChromaKey(activeClip),
        clip: activeClip,
        track: activeTrack,
        isPreload: false,
      },
      deckB: {
        mediaId: nextMedia?.id ?? null,
        src: nextSrc,
        currentTime: nextTime,
        visible: false, // Preloaded, hidden until cut point
        style: { opacity: 0, pointerEvents: "none" },
        vignette: getClipVignette(nextClip ?? null),
        chromaKey: getClipChromaKey(nextClip ?? null),
        clip: nextClip ?? null,
        track: activeTrack,
        isPreload: true,
      },
      activeDeck: "A",
      inTransition: false,
      transitionKind: null,
      activeAudioClips,
      activeTitleClip,
      activeSubtitle,
      hasVideoMedia: true,
    };
  }

  // Case C: Timeline Gap (No video clip at playhead)
  // Look ahead for the upcoming video clip across tracks to preload it
  let upcomingClip: Clip | null = null;
  let upcomingTrack: Track | null = null;

  for (const track of videoTracks) {
    for (const clip of track.clips) {
      if (clip.timeline_start > playhead) {
        if (!upcomingClip || clip.timeline_start < upcomingClip.timeline_start) {
          upcomingClip = clip;
          upcomingTrack = track;
        }
      }
    }
  }

  const upcomingMedia = upcomingClip ? mediaList.find((m) => m.id === upcomingClip.media_id) : null;
  const upcomingSrc = resolveMediaStreamUrl(upcomingMedia, preparedPreviews, mediaServerPort, mediaServerToken);
  const upcomingTime = upcomingClip ? Math.max(0, upcomingClip.source_in / TICKS_PER_SECOND) : 0;

  return {
    playhead,
    sequenceDuration,
    deckA: {
      mediaId: null,
      src: "",
      currentTime: 0,
      visible: false,
      style: { opacity: 0 },
      vignette: null,
      chromaKey: null,
      clip: null,
      track: null,
      isPreload: false,
    },
    deckB: {
      mediaId: upcomingMedia?.id ?? null,
      src: upcomingSrc,
      currentTime: upcomingTime,
      visible: false, // Preloaded during gap, ready for cut arrival
      style: { opacity: 0, pointerEvents: "none" },
      vignette: getClipVignette(upcomingClip ?? null),
      chromaKey: getClipChromaKey(upcomingClip ?? null),
      clip: upcomingClip,
      track: upcomingTrack,
      isPreload: true,
    },
    activeDeck: "A",
    inTransition: false,
    transitionKind: null,
    activeAudioClips,
    activeTitleClip,
    activeSubtitle,
    hasVideoMedia: false,
  };
}

export interface SpeechWord {
  word: string;
  startTicks: number;
  endTicks: number;
  confidence: number;
  isFiller: boolean;
}

export interface SpeechSegment {
  id: number;
  startTicks: number;
  endTicks: number;
  text: string;
  speaker?: string | null;
  confidence: number;
  words: SpeechWord[];
}

export interface SilenceInterval {
  startTicks: number;
  endTicks: number;
  durationMs: number;
}

export interface RippleDeleteReport {
  deletedRangesCount: number;
  totalDurationRemovedTicks: number;
  tracksAffected: number;
  clipsModified: number;
  subtitlesAdjusted: number;
}

export interface PresentationMetrics {
  frameIndex: number;
  presentationTimeUs: number;
  fpsMeasured: number;
  bandwidthGbps: number;
  surfaceWidth: number;
  surfaceHeight: number;
  droppedFrames: number;
  isHardwareAccelerated: boolean;
}

export type InteropFormat = "edl" | "fcpxml" | "premiere_xml" | "otio";

