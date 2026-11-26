# Easy Edits Pro — Design System & UI Specification

**Status:** **FINAL & LOCKED** (Shared Zvirox Linear Solid UI · Orange Accent · `colors/linear-palette`)
**Authority:** Paired with `apps/saturn-tauri/ui/src/styles/linear-theme.css` (shared palette across Zvirox desktop apps) and `apps/saturn-tauri/ui/src/styles/app-accent.css` (Orange accent tokens). Do not introduce off-palette hexes, green UI status hues, frosted glass, dark-mode shadows, or dashboard charting libraries.

---

## 1. Visual Philosophy & Core Invariants

Easy Edits Pro (EEP) uses the shared **Linear Solid Matte** design system tuned for extended non-linear video editing and high-density timeline inspection:

1. **100% Solid Matte Surfaces (Zero Frosted Glass):**
   - Zero `backdrop-filter: blur()`, zero `backdrop-blur-*`, zero `bg-*/10` translucent panel fills, zero ambient glow, and zero inset highlights.
   - Translucent color values are permitted **only** inside the four base variables in `linear-theme.css`: `--shadow-card` (light mode hairline shadow), `--scrim` (modal backdrop), `--chart-fade-up`, and `--chart-fade-down`.
2. **Hard-Enforced Accent Hierarchy:**
   - Primary actions, active selections, focus rings, and EEP identity use the **Orange** accent family (`--primary`, `--primary-hover`, `--primary-fg`, `--primary-text`, `--primary-tint`).
   - Healthy, synced, running, completed, and positive delta indicators use **Sky** (`--sky`, `--sky-text`, `--chip-sky`) — **never green** (`75°–170°` hue is banned across UI chrome and status indicators).
   - Foreign app primary colors (`#15803D`, `#22D3EE`, `#3B82F6`, `#A855F7`, `#EAB308`, etc.) are prohibited in UI styling.
3. **Tokenized Card Elevation Only (`--shadow-card`):**
   - `--shadow-card` is `0 12px 28px -6px rgba(0,0,0,0.5)` in `[data-theme="dark"]` and `0 1px 2px rgb(16 24 40 / 5%)` in `[data-theme="light"]`.
   - All elevated containers must reference `box-shadow: var(--shadow-card)` (or omit `box-shadow`). Never use arbitrary `box-shadow` literals, `drop-shadow`, `shadow-sm/md/lg/xl/2xl/inner`, glow variables, inset shadows, or `shadow-[...]`.
4. **Minimal Precision Rounding:**
   - Cards, panels, and dialogs: `8px` (`--radius-card`)
   - Controls, buttons, inputs, tabs: `6px` (`--radius-control`)
   - Chips, badges, tags, tooltips: `4px` (`--radius-chip`)
   - `rounded-full` is restricted exclusively to small circular status dots, timeline playhead handles, and avatars.
5. **Typography & Motion:**
   - Self-hosted variable **Inter** (`ui/public/fonts/InterVariable.woff2`) via `@font-face` (`--font-sans`) and monospace tabular numeric figures (`--font-mono`) for timecodes, dB levels, keyframes, and frame counts.
   - Transitions use `150ms ease-out` on discrete non-layout properties (`color`, `background-color`, `border-color`, `opacity`, `transform`). `transition: all` is strictly forbidden, `prefers-reduced-motion` is honored, and the timeline playhead never uses CSS transitions.
6. **Raw Media & Technical Surfaces:**
   - Program Monitor, Source Monitor, Web Review proxy player, Canvas Video Scopes (`VideoScopes.tsx`), and audio level meters render raw pixels with zero glass overlays, gradients, or blur filters.
7. **Charting Removal:**
   - Dashboard charting libraries (`echarts`, `echarts-linear-theme.ts`, `DonutLegend`) are removed. Summary telemetry (`StatCard`, `ProgressRow`, `StatusPill`, `DeliverStatsStrip`, `DataTable`) renders directly with CSS tokens.

---

## 2. Token File Architecture

| File | Responsibility | Rules |
| :--- | :--- | :--- |
| `ui/src/styles/linear-theme.css` | Shared base palette (Light & Dark neutrals, status colors, `--shadow-card`, `--scrim`) | Byte-for-byte identical across Zvirox suite (`sha256: a542c03e83fd42bff20815cac21e37182261ebb86f1ca6b04f2341b0c2b21a4e`). Never edit per-app. |
| `ui/src/styles/app-accent.css` | EEP **Orange** primary scale (`--primary`, `--primary-hover`, `--primary-fg`, `--primary-text`, `--primary-tint`) | Only styling token file that differs between apps. Zero green hues. |
| `ui/src/styles/linear-layout.css` | Tailwind v4 `@theme inline` utility bindings (`--color-*`, `--font-*`, `--radius-*`), `@font-face` Inter, spacing, motion | Zero color literals; references variables from `linear-theme.css` and `app-accent.css`. |
| `ui/src/styles/01-editor.css` | Legacy compatibility semantic aliases (`--bg-canvas`, `--bg-elevated`, `--status-success`, `--neutral-*`, etc.) | Maps strictly via `var(--...)` to Linear tokens; zero literal hex/rgb/hsl. |
| `ui/src/styles/00-tokens-glass.css` | Shell/glass compatibility aliases (`--accent-text`, `--glass-l1..l4`, zeroed `--blur-*` = `0px`) | Maps strictly via `var(--...)`; zero blur or translucency. |
| `ui/src/styles/themeColors.ts` | Runtime token resolver (`readThemeToken()`) for `<canvas>` components (`VideoScopes`, `GraphicsPanel`) | Reads computed styles from `document.documentElement` with no hardcoded hex fallbacks. |

---

## 3. Exact Palette Tables

### 3.1 Easy Edits Pro Primary Accent (`app-accent.css`)

| Token | Light Theme | Dark Theme | Purpose |
| :--- | :--- | :--- | :--- |
| `--primary` | `#C2410C` | `#F97316` | Primary action fill, active borders, toggle tracks, key focus accents |
| `--primary-hover` | `#AB390B` | `#FA8432` | Primary hover fill and dark burnt-orange hover feedback |
| `--primary-fg` | `#FFFFFF` | `#1C1C1C` | Text & icons placed directly on `--primary` / `--primary-hover` fills |
| `--primary-text` | `#C2410C` | `#FB923C` | Orange accent label/link text over neutral/tinted surfaces |
| `--primary-tint` | `#FBF2EE` | `#3B281B` | Selected card/row fill and tinted badge/chip background |

> **Compatibility `--accent-text` note:** In `00-tokens-glass.css`, legacy `--accent-text` maps to `var(--primary-text)` (`#FB923C`) in dark mode and `var(--primary-hover)` (`#AB390B`) in light mode so every legacy rule passes WCAG AA (`>= 4.5:1`) even on `#E5E7EB` (`--bg-active`).

### 3.2 Shared Surfaces & Borders (`linear-theme.css`)

| Token | Light Theme | Dark Theme | Usage |
| :--- | :--- | :--- | :--- |
| `--bg-app` | `#F8F9FA` | `#1C1C1C` | Application root viewport and workspace backdrop |
| `--bg-sidebar` | `#F1F3F5` | `#171717` | Collapsible workspace sidebar and rail chrome |
| `--bg-surface` | `#FFFFFF` | `#242424` | Primary cards, panels, dialogs, dock containers |
| `--bg-surface-2` | `#F3F3F3` | `#2D2D2D` | Secondary inputs, subpanels, inset headers, control tracks |
| `--bg-active` | `#E5E7EB` | `#292929` | Pressed controls and active toggle pills |
| `--bg-hover` | `#EFEFEF` | `#242424` | Row and menu item hover states |
| `--bg-track` | `#E5E7EB` | `#2E2E2E` | Slider tracks, progress bars, timeline lane troughs |
| `--bg-bar-idle` | `#DEE2E6` | `#2A2A2A` | Inactive meter segments and idle bars |
| `--border` | `#DEE2E6` | `#2F2F2F` | Standard `1px` crisp structural dividers and card borders |
| `--border-strong` | `#C6CDD4` | `#3B3B3B` | Emphasized input borders, splitter handles, grid lines |
| `--chart-grid` | `#E5E7EB` | `#2A2A2A` | Scope graticules, curve grids, and waveform baselines |
| `--shadow-card` | `0 1px 2px rgb(16 24 40 / 5%)` | `0 12px 28px -6px rgba(0,0,0,0.5)` | Card & popover elevation (sole permitted `box-shadow` token) |
| `--scrim` | `rgba(15, 23, 42, 0.35)` | `rgba(0, 0, 0, 0.6)` | Modal overlay backdrop |

### 3.3 Shared Typography & Role Semantic Palette (`linear-theme.css`)

| Token | Light Theme | Dark Theme | Usage |
| :--- | :--- | :--- | :--- |
| `--text` | `#1C1C1C` | `#E6E6E6` | Primary headings, labels, active values |
| `--text-secondary` | `#495057` | `#B3B3B3` | Secondary descriptions, parameter names |
| `--text-muted` | `#636970` | `#8F8F8F` | Captions, inactive tabs, metadata (`>= 4.5:1` on surface) |
| `--text-faint` | `#8C939A` | `#666666` | Hairline separators and structural placeholder glyphs |
| `--sky` / `--sky-text` | `#0284C7` / `#0369A1` | `#38BDF8` / `#38BDF8` | **Positive / Healthy / Synced** status, luma/waveform channel, markers |
| `--chip-sky` | `#E0F2FE` | `#1F2D38` | Solid chip background for positive/ready status pills |
| `--amber` | `#B45309` | `#F59E0B` | **Warning / Pending / Mid-tone** status, out-markers, audio caution |
| `--red` / `--red-text` | `#E11D48` / `#BE123C` | `#FB7185` / `#FDA4AF` | **Danger / Error / Clipping / Playhead** indicator |
| `--chip-red` | `#FFE4E6` | `#382026` | Solid chip background for error/blocked states |
| `--blue` | `#2563EB` | `#60A5FA` | Secondary informational & blue vectorscope/curve channel |
| `--orange` | `#C2410C` | `#F97316` | Orange accent slot & secondary series |
| `--purple` | `#6D28D9` | `#A78BFA` | Timecode accent, node links, agent/compositing indicators |
| `--pink` | `#DB2777` | `#F472B6` | Audio/video lane badges and label group accents |
| `--gray` | `#6B7280` | `#9CA3AF` | Neutral/offline/muted label accents |
| `--close-hover` | `#DC2626` | `#E5484D` | Window close button hover fill |

---

## 4. Desktop Shell, Workspaces & Canvas Video Scopes

- **Frameless Window Chrome (`TitleBar.tsx` + `tauri.conf.json`):** `1280×800` default (`1024×640` minimum), `decorations: false`, `transparent: false`. The 40 px custom title bar reads the version badge dynamically from Tauri/package metadata, provides the centered drag region with double-click maximize, hosts the **exclusive** Dark/Light `ThemeToggle` (`ui-theme` / `data-theme="dark|light"`), and renders native window minimize / maximize / close buttons.
- **Unified Navigation (`EditorShellFrame`, `Sidebar`, `Breadcrumb`, `WorkspaceShell`):**
  - Collapsible workspace `Sidebar` (`240px` expanded, `56px` auto-collapsed at `<= 1160px` viewport width).
  - Fixed two-row NLE viewport keeping Top-Center Program Monitor, Bottom-Left Tool Rail, Bottom-Center Timeline (`44px` compact track headers), Bottom-Right Master Meter, and Status Bar mounted across NLE suites (`Welcome`, `Media`, `Edit`, `Fusion`, `Color`, `Fairlight`, `Effects`, `Keyframes`, `Deliver`) and self-hosted Web Desk (`?mode=desk` / `?desk=1`).
- **Canvas Video Scopes (`ui/src/components/VideoScopes.tsx`):**
  - Preserved HTML5 `<canvas>` rendering for **Waveform**, **Vectorscope**, **RGB Parade**, **Histogram**, and **Quad** views.
  - Dynamically reads CSS custom properties at paint time (`--bg-app`, `--border-strong`, `--text`, `--text-muted`, `--sky`, `--amber`, `--red`, `--primary`) using `readThemeToken()` + `ctx.globalAlpha` so theme switching repaints scopes cleanly without hardcoded hex strings or green RGB trace channels (`--sky` stands in for luminance/G trace legibility, `--red` for red, `--amber`/`--primary` for chroma/yellow).

---

## 5. Automated UI Gates & Regression Suite

Every UI and styling change must pass the locked gate stack before completion:

- `pnpm --dir apps/saturn-tauri check:linear`
  - Verifies SHA-256 byte-for-byte parity of `linear-theme.css` against the Zvirox baseline and exact EEP Orange `app-accent.css` values.
  - Scans all UI sources (`ui/index.html` and `ui/src/`) for raw hex/`rgb()`/`hsl()`/Tailwind color utilities, retired palette hexes, green hues (`75°–170°`), foreign primaries, banned blur/translucent/glow/inset/non-token shadows, and banned `echarts` imports.
  - Runs `check-contrast.mjs` (**304** semantic text/surface & on-accent WCAG AA >= 4.5:1 pairs), `check-styles.mjs` (bans `transition: all`, layout animation, playhead animation, >800-line CSS/TS files), `check-token-freeze.mjs` (37 frozen color tokens), and `build-token-docs.mjs --check` (`docs/glass-tokens.md`).
- `pnpm --dir apps/saturn-tauri test:e2e` — 13-area Playwright E2E suite verifying zero document overflow, zero card overlap, responsive docks, Web Desk, and workspace geometry invariance across `900×620` to `1920×1080`.

---

*UI Design Final & Locked — Zvirox / Easy Edits Pro (`colors/linear-palette`).*
