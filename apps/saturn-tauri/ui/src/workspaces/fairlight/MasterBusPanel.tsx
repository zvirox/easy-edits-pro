import { InteriorButton } from "../../components/interior/interior-button";
import { formatDb } from "../../utils/formatters";

interface MasterBusPanelProps {
  programPlaying: boolean;
  masterVu: { left: number; right: number; peakDb: number };
  masterGr: number;
  masterGainDraft: number;
  setMasterGainDraft: (val: number) => void;
  perform: (command: string, payload: Record<string, unknown>) => Promise<unknown>;
}

export function MasterBusPanel({
  programPlaying,
  masterVu,
  masterGr,
  masterGainDraft,
  setMasterGainDraft,
  perform,
}: MasterBusPanelProps) {
  return (
    <>
      <div className="audio-master-bus-card">
        <div className="master-bus-header">
          <div className="master-bus-title">
            <strong>MASTER OUTPUT BUS</strong>
            <span className="bus-badge">Stereo Main · 48 kHz</span>
          </div>
          <span className={`mixer-live-badge ${programPlaying ? "playing" : "stopped"}`}>
            {programPlaying ? "LIVE" : "STANDBY"}
          </span>
        </div>

        <div className="master-bus-controls">
          {/* Dual Stereo Master VU Meter */}
          <div className="master-vu-column">
            <div className="vu-meter-pair master-vu-pair">
              <div className="vu-bar-slot">
                <div
                  className="vu-bar-fill left"
                  style={{ height: `${Math.min(100, (masterVu.left || 0) * 100)}%` }}
                />
              </div>
              <div className="vu-bar-slot">
                <div
                  className="vu-bar-fill right"
                  style={{ height: `${Math.min(100, (masterVu.right || 0) * 100)}%` }}
                />
              </div>
            </div>
            <div className="vu-db-readout">{formatDb(masterVu.peakDb)}</div>
          </div>

          {/* Master Vertical Fader */}
          <div className="master-fader-column">
            <div className="fader-scale-ticks">
              <span>+6</span>
              <span className="unity">0</span>
              <span>-6</span>
              <span>-12</span>
              <span>-24</span>
              <span>-48</span>
              <span>-∞</span>
            </div>
            <div className="fader-track-container">
              <input
                type="range"
                className="vertical-fader master-fader"
                min="-60"
                max="6"
                step="0.5"
                value={masterGainDraft}
                onChange={(e) => {
                  const val = Number(e.target.value);
                  setMasterGainDraft(val);
                }}
                onPointerUp={(e) => void perform("bus.set_gain_db", { target: "master", gain_db: Number(e.currentTarget.value) })}
              />
            </div>
            <div className="strip-fader-footer">
              <InteriorButton
                type="button"
                className="strip-unity-btn master-unity"
                title="Reset master gain to unity 0 dB"
                onClick={() => {
                  setMasterGainDraft(0);
                  void perform("bus.set_gain_db", { target: "master", gain_db: 0 });
                }}
              >
                0 dB
              </InteriorButton>
              <span className="strip-gain-text master-gain-text">{formatDb(masterGainDraft)}</span>
            </div>
          </div>
        </div>
      </div>

      {/* Master True Peak Limiter Module */}
      <div className="audio-master-limiter-card">
        <div className="limiter-card-header">
          <div className="limiter-title">
            <strong>TRUE PEAK LIMITER</strong>
            <span className="limiter-badge">-0.5 dBFS CEILING</span>
          </div>
          <span className="limiter-active-pill">BRICKWALL</span>
        </div>
        <div className="limiter-card-body">
          <div className="limiter-specs">
            <span>Ceiling: <b>-0.5 dB</b></span>
            <span>Ratio: <b>20:1</b></span>
            <span>Attack: <b>1.0 ms</b></span>
          </div>
          <div className="limiter-gr-container">
            <div className="gr-label-row">
              <span>Master GR</span>
              <b className={masterGr < -0.1 ? "gr-active" : ""}>
                {masterGr < -0.05 ? `${masterGr.toFixed(1)} dB` : "0.0 dB"}
              </b>
            </div>
            <div className="gr-meter-bar-slot">
              <div
                className="gr-meter-bar-fill master"
                style={{ width: `${Math.min(100, Math.max(0, -masterGr * 5))}%` }}
              />
            </div>
          </div>
        </div>
      </div>
    </>
  );
}
