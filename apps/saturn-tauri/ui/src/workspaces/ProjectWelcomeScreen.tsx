import { LoadingButton } from "../components/interior/loading-button";
import {
  Plus,
  FolderOpen,
  ArrowRight,
  ArrowLeft,
  RotateCcw,
  Film,
  Clock,
} from "lucide-react";
import {
  type AppStage,
  type RecentProject,
  type RecoveryStatus,
  resolutionPresets,
  frameRateOptions,
} from "../types";

export interface ProjectWelcomeScreenProps {
  appStage: AppStage;
  loadingMessage: string;
  connection: string;
  welcomeMode: "home" | "new-project";
  setWelcomeMode: (mode: "home" | "new-project") => void;
  setupProjectName: string;
  setSetupProjectName: (val: string) => void;
  setupSequenceName: string;
  setSetupSequenceName: (val: string) => void;
  setupResolution: string;
  setSetupResolution: (val: string) => void;
  setupWidth: string;
  setSetupWidth: (val: string) => void;
  setupHeight: string;
  setSetupHeight: (val: string) => void;
  setupFrameRate: string;
  setSetupFrameRate: (val: string) => void;
  recentProjects: RecentProject[];
  recoveryStatus: RecoveryStatus;
  notice: string | null;
  setNotice: (msg: string) => void;
  openProject: (path?: string) => Promise<void> | void;
  createProjectFromWelcome: () => Promise<void>;
  recoverProjectFromWelcome: () => Promise<void>;
}

export function ProjectWelcomeScreen({
  appStage,
  loadingMessage,
  connection,
  welcomeMode,
  setWelcomeMode,
  setupProjectName,
  setSetupProjectName,
  setupSequenceName,
  setSetupSequenceName,
  setupResolution,
  setSetupResolution,
  setupWidth,
  setSetupWidth,
  setupHeight,
  setSetupHeight,
  setupFrameRate,
  setSetupFrameRate,
  recentProjects,
  recoveryStatus,
  notice,
  setNotice,
  openProject,
  createProjectFromWelcome,
  recoverProjectFromWelcome,
}: ProjectWelcomeScreenProps) {
  if (appStage === "loading") {
    return (
      <main className="project-loading-screen" aria-live="polite" aria-busy="true">
        <img src="/saturn-camera.png" alt="" />
        <div className="loading-orbit">
          <span />
          <span />
          <span />
        </div>
        <p className="welcome-kicker">EASY EDITS PRO</p>
        <h1>{loadingMessage}</h1>
        <p className="welcome-muted">Getting your workspace ready</p>
        <div className="loading-track">
          <i />
        </div>
      </main>
    );
  }

  return (
    <main className="welcome-shell">
      <div className="welcome-ambient-glow" aria-hidden="true" />
      <div className="welcome-grid-pattern" aria-hidden="true" />
      <header className="welcome-topbar">
        <div className="welcome-brand">
          <img src="/saturn-camera.png" alt="" />
          <span>
            <strong>Easy Edits Pro</strong>
            <small>VIDEO EDITING WORKSPACE</small>
          </span>
        </div>
        <div className="welcome-status">
          <i className={connection.includes("error") ? "offline" : "online"} />
          {connection.includes("error") ? "Core unavailable" : "Ready to edit"}
        </div>
      </header>
      <div className="welcome-layout">
        <section className="welcome-hero">
          <div className="welcome-hero-copy">
            <p className="welcome-kicker">
              PROJECT STUDIO <span>·</span> {new Date().getFullYear()}
            </p>
            <h1>
              Make room<br />for the <em>next cut.</em>
            </h1>
            <p className="welcome-intro">
              Pick up where you left off or shape a fresh timeline, tuned to the way you shoot.
            </p>
          </div>
          <div className="welcome-editor-art" aria-hidden="true">
            <div className="welcome-art-toolbar">
              <span />
              <span />
              <span />
              <i />
              <b>PROGRAM</b>
              <small>00:00:12:08</small>
            </div>
            <div className="welcome-art-preview">
              <span className="art-sun" />
              <span className="art-horizon" />
              <span className="art-caption">A FRAME OF YOUR OWN</span>
            </div>
            <div className="welcome-art-timeline">
              <div className="art-ruler">
                <span>00:00</span>
                <span>00:05</span>
                <span>00:10</span>
                <span>00:15</span>
              </div>
              <div className="art-track">
                <b>V1</b>
                <i />
                <i />
              </div>
              <div className="art-track audio">
                <b>A1</b>
                <i />
              </div>
              <span className="art-playhead" />
            </div>
          </div>
          <div className="welcome-hero-foot">
            <span>01 <i /> Shape the sequence</span>
            <span>02 <i /> Find your rhythm</span>
            <span>03 <i /> Make it yours</span>
          </div>
        </section>

        <section className="welcome-panel" aria-label="Project selection">
          {welcomeMode === "home" ? (
            <>
              <div className="welcome-panel-heading">
                <p className="welcome-kicker">YOUR WORKSPACE</p>
                <h2>Welcome back.</h2>
                <p>Start something new or continue a project.</p>
              </div>
              <div className="welcome-actions">
                <button
                  className="welcome-action-card new"
                  onClick={() => {
                    setSetupProjectName("Untitled project");
                    setSetupSequenceName("Sequence 1");
                    setWelcomeMode("new-project");
                  }}
                >
                  <span className="welcome-action-icon">
                    <Plus className="size-4" />
                  </span>
                  <span>
                    <strong>Create a new project</strong>
                    <small>Choose a format and start editing</small>
                  </span>
                  <ArrowRight className="welcome-action-arrow" aria-hidden="true" />
                </button>
                <LoadingButton
                  className="welcome-action-card open"
                  onAction={() => openProject()}
                  onError={(error) => setNotice(String(error))}
                  pendingLabel="Opening project…"
                  successLabel="Project ready"
                  errorLabel="Could not open project"
                >
                  <span className="welcome-action-icon folder">
                    <FolderOpen className="size-4" />
                  </span>
                  <span>
                    <strong>Open an existing project</strong>
                    <small>Browse for a .saturn project file</small>
                  </span>
                  <ArrowRight className="welcome-action-arrow" aria-hidden="true" />
                </LoadingButton>
              </div>
              {recoveryStatus.available && (
                <div className="welcome-recovery">
                  <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
                    <RotateCcw className="size-4 text-amber-400 shrink-0" />
                    <span>
                      <strong>Recovered session available</strong>
                      <small>{recoveryStatus.projectName ?? "Autosaved project"} · autosave found</small>
                    </span>
                  </div>
                  <LoadingButton
                    className="welcome-recover-button"
                    onAction={recoverProjectFromWelcome}
                    onError={(error) => setNotice(String(error))}
                    pendingLabel="Restoring…"
                    successLabel="Restored"
                    errorLabel="Restore failed"
                  >
                    Recover
                  </LoadingButton>
                </div>
              )}
              <div className="welcome-recent-heading">
                <h3>Recent projects</h3>
                <span>{recentProjects.length ? `${recentProjects.length} saved here` : "Ready when you are"}</span>
              </div>
              {recentProjects.length ? (
                <div className="welcome-recent-list">
                  {recentProjects.map((item) => (
                    <LoadingButton
                      key={item.path}
                      className="welcome-recent-item"
                      onAction={() => openProject(item.path)}
                      onError={(error) => setNotice(String(error))}
                      pendingLabel="Opening…"
                      successLabel="Opened"
                      errorLabel="Could not open"
                    >
                      <span className="recent-project-icon">
                        <Film className="size-3.5" />
                      </span>
                      <span className="recent-project-name">
                        <strong>{item.name}</strong>
                        <small title={item.path}>{item.path}</small>
                      </span>
                      <ArrowRight className="recent-arrow" aria-hidden="true" />
                    </LoadingButton>
                  ))}
                </div>
              ) : (
                <div className="welcome-recent-empty">
                  <span><Clock className="size-4" /></span>
                  <p>Your recent projects will appear here.</p>
                </div>
              )}
            </>
          ) : (
            <>
              <button className="welcome-back" onClick={() => setWelcomeMode("home")}>
                <ArrowLeft className="size-3.5" /> <span>Back to projects</span>
              </button>
              <div className="welcome-panel-heading setup-heading">
                <p className="welcome-kicker">NEW PROJECT</p>
                <h2>Set your canvas.</h2>
                <p>Choose the timeline format for this edit.</p>
              </div>
              <form
                className="project-setup-form"
                onSubmit={(event) => {
                  event.preventDefault();
                  void createProjectFromWelcome().catch((error: unknown) => setNotice(String(error)));
                }}
              >
                <label>
                  Project name
                  <input
                    autoFocus
                    required
                    maxLength={120}
                    value={setupProjectName}
                    onChange={(event) => setSetupProjectName(event.target.value)}
                    placeholder="Untitled project"
                  />
                </label>
                <label>
                  Timeline name
                  <input
                    required
                    maxLength={120}
                    value={setupSequenceName}
                    onChange={(event) => setSetupSequenceName(event.target.value)}
                    placeholder="Sequence 1"
                  />
                </label>
                <div className="setup-field-row">
                  <label>
                    Resolution
                    <select
                      value={setupResolution}
                      onChange={(event) => {
                        const value = event.target.value;
                        setSetupResolution(value);
                        const preset = resolutionPresets.find((item) => item.value === value);
                        if (preset && value !== "custom") {
                          setSetupWidth(String(preset.width));
                          setSetupHeight(String(preset.height));
                        }
                      }}
                    >
                      {resolutionPresets.map((preset) => (
                        <option value={preset.value} key={preset.value}>
                          {preset.label}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label>
                    Frame rate
                    <select
                      value={setupFrameRate}
                      onChange={(event) => setSetupFrameRate(event.target.value)}
                    >
                      {frameRateOptions.map((option) => (
                        <option value={option.value} key={option.value}>
                          {option.label}
                        </option>
                      ))}
                    </select>
                  </label>
                </div>
                {setupResolution === "custom" && (
                  <div className="setup-field-row custom-resolution">
                    <label>
                      Width
                      <input
                        required
                        type="number"
                        min="1"
                        max="16384"
                        value={setupWidth}
                        onChange={(event) => setSetupWidth(event.target.value)}
                      />
                    </label>
                    <span>×</span>
                    <label>
                      Height
                      <input
                        required
                        type="number"
                        min="1"
                        max="16384"
                        value={setupHeight}
                        onChange={(event) => setSetupHeight(event.target.value)}
                      />
                    </label>
                  </div>
                )}
                <div className="setup-summary">
                  <span>PROJECT PROFILE</span>
                  <strong>
                    {setupWidth} × {setupHeight}
                    <i />
                    {frameRateOptions.find((option) => option.value === setupFrameRate)?.label}
                  </strong>
                  <small>These settings define playback and export for your sequence.</small>
                </div>
                {notice && (
                  <p className="welcome-notice" role="alert">
                    {notice}
                  </p>
                )}
                <div className="setup-actions">
                  <button
                    type="button"
                    className="welcome-secondary-button"
                    onClick={() => setWelcomeMode("home")}
                  >
                    Cancel
                  </button>
                  <LoadingButton
                    className="welcome-create-button"
                    onAction={createProjectFromWelcome}
                    onError={(error) => setNotice(String(error))}
                    pendingLabel="Creating project…"
                    successLabel="Project created"
                    errorLabel="Could not create"
                  >
                    Create project <span>→</span>
                  </LoadingButton>
                </div>
              </form>
            </>
          )}
          {welcomeMode === "home" && notice && (
            <p className="welcome-notice" role="status">
              {notice}
            </p>
          )}
          <footer className="welcome-panel-foot">
            <span>
              PROJECT FORMAT <b>.SATURN</b>
            </span>
            <span>LOCAL · PRIVATE · YOURS</span>
          </footer>
        </section>
      </div>
    </main>
  );
}
