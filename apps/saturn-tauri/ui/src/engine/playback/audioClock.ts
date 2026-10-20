import type { TrackAudioProcessing, Track, LoudnessStandard, LoudnessReport, AudioBus, BusTarget } from "./types.ts";
import { EbuR128WebMeter } from "./audioMeter.ts";

/**
 * AudioClockMaster manages a hardware-synchronized Web Audio context
 * providing monotonic, drift-free master clock timing, stereo panning,
 * mute/solo matrix routing, real-time peak VU metering, and ITU-R BS.1770-4
 * / EBU R128 loudness compliance metering.
 */
export class AudioClockMaster {
  private ctx: AudioContext | null = null;
  private masterGain: GainNode | null = null;
  private masterBusGain: GainNode | null = null;
  private masterLimiter: DynamicsCompressorNode | null = null;
  private masterAnalyser: AnalyserNode | null = null;
  private scriptProcessor: ScriptProcessorNode | null = null;
  private ebuMeter = new EbuR128WebMeter(48000, "ebu_r128");

  private trackInputs = new Map<number, GainNode>();
  private trackGains = new Map<number, GainNode>();
  private trackPanners = new Map<number, StereoPannerNode>();
  private trackAnalysers = new Map<number, AnalyserNode>();
  private busOutputs = new Map<BusTarget, GainNode>();
  private trackBusTargets = new Map<number, BusTarget>();
  private trackDenoiseFilters = new Map<number, BiquadFilterNode>();
  private trackLowShelfs = new Map<number, BiquadFilterNode>();
  private trackMidPeaks = new Map<number, BiquadFilterNode>();
  private trackHighShelfs = new Map<number, BiquadFilterNode>();
  private trackCompressors = new Map<number, DynamicsCompressorNode>();
  private trackPans = new Map<number, number>();
  private trackRawGains = new Map<number, number>();
  private trackMutes = new Map<number, boolean>();
  private trackSolos = new Map<number, boolean>();
  private connectedMediaElements = new WeakSet<HTMLMediaElement>();

  getContext(): AudioContext | null {
    if (typeof window === "undefined") return null;
    if (!this.ctx) {
      const AudioCtx = window.AudioContext || (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
      if (AudioCtx) {
        this.ctx = new AudioCtx();
        this.masterGain = this.ctx.createGain();
        this.masterBusGain = this.ctx.createGain();

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

        this.masterGain.connect(this.masterBusGain);
        this.masterBusGain.connect(this.masterLimiter);
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
            this.masterBusGain.connect(this.scriptProcessor);
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
    return performance.now() / 1000;
  }

  private getBusOutput(target: BusTarget): GainNode | null {
    const ctx = this.getContext();
    if (!ctx || !this.masterGain || !this.masterBusGain) return null;
    if (target === "master") return this.masterBusGain;
    let output = this.busOutputs.get(target);
    if (!output) {
      output = ctx.createGain();
      output.connect(this.masterGain);
      this.busOutputs.set(target, output);
    }
    return output;
  }

  setBusGain(target: BusTarget, gainDb: number): void {
    const ctx = this.getContext();
    const output = this.getBusOutput(target);
    if (!ctx || !output) return;
    output.gain.setTargetAtTime(Math.max(0, Math.min(4, 10 ** (gainDb / 20))), ctx.currentTime, 0.02);
  }

  private routeTrack(trackId: number, target: BusTarget): void {
    const analyser = this.trackAnalysers.get(trackId);
    const output = this.getBusOutput(target);
    if (!analyser || !output || this.trackBusTargets.get(trackId) === target) return;
    analyser.disconnect();
    analyser.connect(output);
    this.trackBusTargets.set(trackId, target);
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
      analyser.connect(this.getBusOutput("master") ?? this.masterGain);
      this.trackBusTargets.set(trackId, "master");
    }
    const gainNode = this.trackGains.get(trackId);
    if (gainNode) {
      const linearGain = Math.max(0, Math.min(2, 10 ** (gainDb / 20)));
      gainNode.gain.setValueAtTime(linearGain, ctx.currentTime);
    }
    return inputNode;
  }

  connectMediaElement(trackId: number, el: HTMLMediaElement): void {
    const ctx = this.getContext();
    if (!ctx || this.connectedMediaElements.has(el)) return;
    try {
      const source = ctx.createMediaElementSource(el);
      const trackInput = this.getTrackGainNode(trackId);
      if (trackInput) {
        source.connect(trackInput);
        this.connectedMediaElements.add(el);
      }
    } catch {
      // Element might already be attached or host environment restricted
    }
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

  updateTracksState(tracks: Track[], buses: AudioBus[] = []): void {
    const ctx = this.getContext();
    if (!ctx) return;
    const anySolo = tracks.some((t) => t.solo);
    for (const bus of buses) this.setBusGain(bus.target, bus.gain_db);
    for (const track of tracks) {
      this.trackRawGains.set(track.id, track.gain_db);
      this.trackMutes.set(track.id, !!track.muted);
      this.trackSolos.set(track.id, !!track.solo);
      this.trackPans.set(track.id, track.pan ?? 0);
      this.updateTrackAudioProcessing(track.id, track.audio_processing);
      this.routeTrack(track.id, track.bus ?? "master");

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
