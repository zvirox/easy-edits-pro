import type { LoudnessReport, LoudnessStandard } from "./types.ts";

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
