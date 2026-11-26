# Color Audit — Easy Edits Pro (`apps/saturn-tauri/ui`)

Baseline source audit generated before the Orange Linear color migration (`colors/linear-palette`). Covers every source hex, `rgb()`/`rgba()`, `hsl()`/`hsla()`, Tailwind palette class, and arbitrary color utility across `ui/index.html` and `ui/src/` outside the final token files (`ui/src/styles/linear-theme.css`, `ui/src/styles/app-accent.css`).

- **Total non-token literal color occurrences:** 378
- **Literal green / teal hue occurrences flagged:** 35
- **Legacy green/teal CSS custom property tokens to migrate to `--sky` / `--sky-text` / `--chip-sky` (or remove):** `--tint-green-bg` (16), `--tint-green-text` (5), `--accent-green` (19), `--tint-green-fg` (6), `--tint-green-border` (4), `--accent-teal` (4), `--tint-teal-bg` (3), `--tint-teal-fg` (3), `--tint-teal-border` (3)
- **Charts to remove:** `echarts` (`apps/saturn-tauri/package.json`), `ui/src/components/linear/echarts-linear-theme.ts`, and `ui/src/components/linear/DonutLegend.tsx`.
- **Canvas scopes kept (owner-confirmed §5):** `ui/src/components/VideoScopes.tsx` (waveform monitor, vectorscope, RGB parade, histogram, quad — repointed to CSS variables using `--sky`, `--amber`, `--red`, `--primary`).

## 1. Literal Green & Teal Uses (Must Remove / Map to Sky or Primary)

| Location | Kind | Value | Flags |
| --- | --- | --- | --- |
| `ui/src/app/useTrackGroupOperations.ts:23` | hex | `#10b981` | retired, GREEN(160deg) |
| `ui/src/components/AudioMonitorMixer.tsx:105` | hex | `#10b981` | retired, GREEN(160deg) |
| `ui/src/components/CustomShaderEditor.tsx:292` | hex | `#68d391` | GREEN(143deg) |
| `ui/src/components/HslQualifierEditor.tsx:175` | hex | `#68d391` | GREEN(143deg) |
| `ui/src/components/HslQualifierEditor.tsx:208` | hex | `#00ff00` | GREEN(120deg) |
| `ui/src/components/RgbCurvesEditor.tsx:253` | hex | `#14532d` | GREEN(144deg) |
| `ui/src/components/StatusBar.tsx:123` | hex | `#10b981` | retired, GREEN(160deg) |
| `ui/src/components/VideoScopes.tsx:327` | hex | `#22c55e` | retired, GREEN(142deg) |
| `ui/src/components/VideoScopes.tsx:395` | hex | `#22c55e` | retired, GREEN(142deg) |
| `ui/src/components/VideoScopes.tsx:534` | hex | `#22c55e` | retired, GREEN(142deg) |
| `ui/src/components/linear/echarts-linear-theme.ts:7` | hex | `#22C55E` | retired, GREEN(142deg) |
| `ui/src/features/agent/AgentCard.tsx:175` | hex | `#10b981` | retired, GREEN(160deg) |
| `ui/src/features/dialogs/GpuTelemetryModal.tsx:47` | hex | `#10b981` | retired, GREEN(160deg) |
| `ui/src/features/dialogs/GpuTelemetryModal.tsx:71` | hex | `#10b981` | retired, GREEN(160deg) |
| `ui/src/features/dialogs/GpuTelemetryModal.tsx:72` | hex | `#10b981` | retired, GREEN(160deg) |
| `ui/src/features/dialogs/MediaBinModal.tsx:55` | hex | `#10b981` | retired, GREEN(160deg) |
| `ui/src/features/dialogs/MediaPropsModal.tsx:116` | hex | `#10b981` | retired, GREEN(160deg) |
| `ui/src/features/inspector/AgentTelemetryDrawer.tsx:217` | hex | `#10b981` | retired, GREEN(160deg) |
| `ui/src/features/inspector/EffectsInspector.tsx:343` | hex | `#00ff00` | GREEN(120deg) |
| `ui/src/features/tasksheet/AgentInbox.tsx:160` | hex | `#10b981` | retired, GREEN(160deg) |
| `ui/src/features/tasksheet/AgentInbox.tsx:161` | hex | `#10b981` | retired, GREEN(160deg) |
| `ui/src/features/tasksheet/TaskSheetPanel.tsx:144` | hex | `#10b981` | retired, GREEN(160deg) |
| `ui/src/features/tasksheet/TaskSheetPanel.tsx:447` | hex | `#10b981` | retired, GREEN(160deg) |
| `ui/src/features/tasksheet/TaskSheetPanel.tsx:450` | hex | `#10b981` | retired, GREEN(160deg) |
| `ui/src/features/tasksheet/TaskSheetPanel.tsx:489` | hex | `#10b981` | retired, GREEN(160deg) |
| `ui/src/features/timeline/AiBranchDiffRibbon.tsx:107` | hex | `#10b981` | retired, GREEN(160deg) |
| `ui/src/features/timeline/TimelineTrackLane.tsx:333` | hex | `#34d399` | GREEN(158deg) |
| `ui/src/features/timeline/TimelineTrackLane.tsx:334` | rgba | `rgba(52, 211, 153, 0.42)` | GREEN(158deg) |
| `ui/src/features/timeline/labelColors.ts:16` | hex | `#166534` | GREEN(143deg) |
| `ui/src/features/timeline/labelColors.ts:17` | hex | `#22c55e` | retired, GREEN(142deg) |
| `ui/src/workspaces/docks/EffectsDocks.tsx:307` | hex | `#00ff00` | GREEN(120deg) |
| `ui/src/workspaces/docks/EffectsDocks.tsx:313` | hex | `#00ff00` | GREEN(120deg) |
| `ui/src/workspaces/docks/FusionDocks.tsx:136` | hex | `#00ff00` | GREEN(120deg) |
| `ui/src/workspaces/docks/FusionDocks.tsx:350` | hex | `#00ff00` | GREEN(120deg) |
| `ui/src/workspaces/fairlight/DspRackPanel.tsx:113` | hex | `#34d399` | GREEN(158deg) |

## 2. All Non-Token Color Literals & Palette Classes

| Location | Kind | Value | Flags |
| --- | --- | --- | --- |
| `ui/index.html:6` | hex | `#050505` | retired |
| `ui/index.html:20` | hex | `#050505` | retired |
| `ui/index.html:20` | hex | `#F3F4F6` | — |
| `ui/src/app/useFileProjectFlows.ts:135` | hex | `#808080` | — |
| `ui/src/app/useMediaOperations.ts:119` | hex | `#be185d` | — |
| `ui/src/app/useTrackGroupOperations.ts:23` | hex | `#be185d` | — |
| `ui/src/app/useTrackGroupOperations.ts:23` | hex | `#10b981` | retired, GREEN(160deg) |
| `ui/src/app/useTrackGroupOperations.ts:23` | hex | `#f59e0b` | retired |
| `ui/src/app/useTrackGroupOperations.ts:23` | hex | `#ec4899` | retired |
| `ui/src/app/useTrackGroupOperations.ts:23` | hex | `#6b21a8` | — |
| `ui/src/app/useTrackGroupOperations.ts:23` | hex | `#6b21a8` | — |
| `ui/src/components/AudioMonitorMixer.tsx:104` | hex | `#be185d` | — |
| `ui/src/components/AudioMonitorMixer.tsx:105` | hex | `#10b981` | retired, GREEN(160deg) |
| `ui/src/components/AudioMonitorMixer.tsx:106` | hex | `#6b21a8` | — |
| `ui/src/components/BackgroundMattingCard.tsx:97` | hex | `#180d17` | — |
| `ui/src/components/BackgroundMattingCard.tsx:98` | hex | `#462d40` | — |
| `ui/src/components/BackgroundMattingCard.tsx:101` | hex | `#e2e8f0` | — |
| `ui/src/components/BackgroundMattingCard.tsx:109` | hex | `#a0aec0` | — |
| `ui/src/components/BackgroundMattingCard.tsx:117` | hex | `#6b21a8` | — |
| `ui/src/components/BackgroundMattingCard.tsx:118` | hex | `#ffffff` | — |
| `ui/src/components/BackgroundMattingCard.tsx:130` | hex | `#cbd5e0` | — |
| `ui/src/components/BackgroundMattingCard.tsx:149` | hex | `#be185d` | — |
| `ui/src/components/BackgroundMattingCard.tsx:149` | hex | `#462d40` | — |
| `ui/src/components/BackgroundMattingCard.tsx:150` | hex | `#462d40` | — |
| `ui/src/components/BackgroundMattingCard.tsx:150` | hex | `#100810` | — |
| `ui/src/components/BackgroundMattingCard.tsx:151` | hex | `#ffffff` | — |
| `ui/src/components/BackgroundMattingCard.tsx:151` | hex | `#cbd5e0` | — |
| `ui/src/components/BackgroundMattingCard.tsx:169` | hex | `#cbd5e0` | — |
| `ui/src/components/BackgroundMattingCard.tsx:181` | hex | `#cbd5e0` | — |
| `ui/src/components/BackgroundMattingCard.tsx:217` | hex | `#462d40` | — |
| `ui/src/components/BackgroundMattingCard.tsx:221` | hex | `#a0aec0` | — |
| `ui/src/components/BackgroundMattingCard.tsx:234` | hex | `#5a4053` | — |
| `ui/src/components/BackgroundMattingCard.tsx:234` | hex | `#6b21a8` | — |
| `ui/src/components/BackgroundMattingCard.tsx:237` | hex | `#ffffff` | — |
| `ui/src/components/CustomShaderEditor.tsx:168` | hex | `#180d17` | — |
| `ui/src/components/CustomShaderEditor.tsx:169` | hex | `#462d40` | — |
| `ui/src/components/CustomShaderEditor.tsx:172` | hex | `#e2e8f0` | — |
| `ui/src/components/CustomShaderEditor.tsx:180` | hex | `#a0aec0` | — |
| `ui/src/components/CustomShaderEditor.tsx:189` | hex | `#9f1249` | — |
| `ui/src/components/CustomShaderEditor.tsx:190` | hex | `#ffffff` | — |
| `ui/src/components/CustomShaderEditor.tsx:204` | hex | `#a0aec0` | — |
| `ui/src/components/CustomShaderEditor.tsx:209` | hex | `#100810` | — |
| `ui/src/components/CustomShaderEditor.tsx:210` | hex | `#5a4053` | — |
| `ui/src/components/CustomShaderEditor.tsx:212` | hex | `#f7fafc` | — |
| `ui/src/components/CustomShaderEditor.tsx:231` | hex | `#100810` | — |
| `ui/src/components/CustomShaderEditor.tsx:232` | hex | `#5a4053` | — |
| `ui/src/components/CustomShaderEditor.tsx:234` | hex | `#e2e8f0` | — |
| `ui/src/components/CustomShaderEditor.tsx:245` | hex | `#342033` | — |
| `ui/src/components/CustomShaderEditor.tsx:246` | hex | `#462d40` | — |
| `ui/src/components/CustomShaderEditor.tsx:252` | hex | `#d9468f` | — |
| `ui/src/components/CustomShaderEditor.tsx:258` | hex | `#cbd5e0` | — |
| `ui/src/components/CustomShaderEditor.tsx:275` | hex | `#8b7284` | — |
| `ui/src/components/CustomShaderEditor.tsx:289` | hex | `#110910` | — |
| `ui/src/components/CustomShaderEditor.tsx:290` | hex | `#462d40` | — |
| `ui/src/components/CustomShaderEditor.tsx:292` | hex | `#68d391` | GREEN(143deg) |
| `ui/src/components/ErrorBoundary.tsx:59` | hex | `#09090b` | retired |
| `ui/src/components/ErrorBoundary.tsx:60` | hex | `#f8fafc` | — |
| `ui/src/components/HslQualifierEditor.tsx:120` | hex | `#180d17` | — |
| `ui/src/components/HslQualifierEditor.tsx:121` | hex | `#462d40` | — |
| `ui/src/components/HslQualifierEditor.tsx:124` | hex | `#e2e8f0` | — |
| `ui/src/components/HslQualifierEditor.tsx:136` | hex | `#a0aec0` | — |
| `ui/src/components/HslQualifierEditor.tsx:136` | hex | `#100810` | — |
| `ui/src/components/HslQualifierEditor.tsx:141` | hex | `#d9468f` | — |
| `ui/src/components/HslQualifierEditor.tsx:141` | hex | `#cbd5e0` | — |
| `ui/src/components/HslQualifierEditor.tsx:148` | hex | `#cbd5e0` | — |
| `ui/src/components/HslQualifierEditor.tsx:160` | hex | `#8b7284` | — |
| `ui/src/components/HslQualifierEditor.tsx:163` | hex | `#462d40` | — |
| `ui/src/components/HslQualifierEditor.tsx:163` | hex | `#ecc94b` | — |
| `ui/src/components/HslQualifierEditor.tsx:169` | hex | `#462d40` | — |
| `ui/src/components/HslQualifierEditor.tsx:169` | hex | `#d9468f` | — |
| `ui/src/components/HslQualifierEditor.tsx:175` | hex | `#462d40` | — |
| `ui/src/components/HslQualifierEditor.tsx:175` | hex | `#68d391` | GREEN(143deg) |
| `ui/src/components/HslQualifierEditor.tsx:181` | hex | `#462d40` | — |
| `ui/src/components/HslQualifierEditor.tsx:181` | hex | `#fc8181` | — |
| `ui/src/components/HslQualifierEditor.tsx:190` | hex | `#100810` | — |
| `ui/src/components/HslQualifierEditor.tsx:190` | hex | `#462d40` | — |
| `ui/src/components/HslQualifierEditor.tsx:193` | hex | `#cbd5e0` | — |
| `ui/src/components/HslQualifierEditor.tsx:197` | hex | `#a0aec0` | — |
| `ui/src/components/HslQualifierEditor.tsx:208` | hex | `#ff0000` | — |
| `ui/src/components/HslQualifierEditor.tsx:208` | hex | `#ffff00` | — |
| `ui/src/components/HslQualifierEditor.tsx:208` | hex | `#00ff00` | GREEN(120deg) |
| `ui/src/components/HslQualifierEditor.tsx:208` | hex | `#00ffff` | — |
| `ui/src/components/HslQualifierEditor.tsx:208` | hex | `#0000ff` | — |
| `ui/src/components/HslQualifierEditor.tsx:208` | hex | `#ff00ff` | — |
| `ui/src/components/HslQualifierEditor.tsx:208` | hex | `#ff0000` | — |
| `ui/src/components/HslQualifierEditor.tsx:217` | hex | `#a0aec0` | — |
| `ui/src/components/HslQualifierEditor.tsx:228` | hex | `#a0aec0` | — |
| `ui/src/components/HslQualifierEditor.tsx:242` | hex | `#100810` | — |
| `ui/src/components/HslQualifierEditor.tsx:242` | hex | `#462d40` | — |
| `ui/src/components/HslQualifierEditor.tsx:245` | hex | `#a0aec0` | — |
| `ui/src/components/HslQualifierEditor.tsx:249` | hex | `#a0aec0` | — |
| `ui/src/components/HslQualifierEditor.tsx:261` | hex | `#a0aec0` | — |
| `ui/src/components/HslQualifierEditor.tsx:275` | hex | `#342033` | — |
| `ui/src/components/HslQualifierEditor.tsx:275` | hex | `#462d40` | — |
| `ui/src/components/HslQualifierEditor.tsx:276` | hex | `#d9468f` | — |
| `ui/src/components/HslQualifierEditor.tsx:282` | hex | `#cbd5e0` | — |
| `ui/src/components/HslQualifierEditor.tsx:294` | hex | `#cbd5e0` | — |
| `ui/src/components/HslQualifierEditor.tsx:306` | hex | `#cbd5e0` | — |
| `ui/src/components/LiveMulticamSwitcher.tsx:75` | hex | `#180d17` | — |
| `ui/src/components/LiveMulticamSwitcher.tsx:76` | hex | `#462d40` | — |
| `ui/src/components/LiveMulticamSwitcher.tsx:79` | hex | `#e2e8f0` | — |
| `ui/src/components/LiveMulticamSwitcher.tsx:87` | hex | `#a0aec0` | — |
| `ui/src/components/LiveMulticamSwitcher.tsx:94` | hex | `#e53e3e` | — |
| `ui/src/components/LiveMulticamSwitcher.tsx:94` | hex | `#a0aec0` | — |
| `ui/src/components/LiveMulticamSwitcher.tsx:101` | hex | `#a0aec0` | — |
| `ui/src/components/LiveMulticamSwitcher.tsx:155` | hex | `#8b7284` | — |
| `ui/src/components/LiveMulticamSwitcher.tsx:161` | hex | `#fff` | — |
| `ui/src/components/LiveMulticamSwitcher.tsx:161` | hex | `#cbd5e0` | — |
| `ui/src/components/LiveMulticamSwitcher.tsx:166` | hex | `#8b7284` | — |
| `ui/src/components/LiveMulticamSwitcher.tsx:175` | hex | `#8b7284` | — |
| `ui/src/components/LiveMulticamSwitcher.tsx:180` | hex | `#e53e3e` | — |
| `ui/src/components/RgbCurvesEditor.tsx:190` | hex | `#462d40` | — |
| `ui/src/components/RgbCurvesEditor.tsx:219` | hex | `#5a4053` | — |
| `ui/src/components/RgbCurvesEditor.tsx:226` | hex | `#8b7284` | — |
| `ui/src/components/RgbCurvesEditor.tsx:231` | hex | `#100810` | — |
| `ui/src/components/RgbCurvesEditor.tsx:238` | hex | `#342033` | — |
| `ui/src/components/RgbCurvesEditor.tsx:239` | hex | `#5a4053` | — |
| `ui/src/components/RgbCurvesEditor.tsx:240` | hex | `#342033` | — |
| `ui/src/components/RgbCurvesEditor.tsx:241` | hex | `#342033` | — |
| `ui/src/components/RgbCurvesEditor.tsx:242` | hex | `#5a4053` | — |
| `ui/src/components/RgbCurvesEditor.tsx:243` | hex | `#342033` | — |
| `ui/src/components/RgbCurvesEditor.tsx:244` | hex | `#342033` | — |
| `ui/src/components/RgbCurvesEditor.tsx:253` | hex | `#5a4053` | — |
| `ui/src/components/RgbCurvesEditor.tsx:253` | hex | `#7f1d1d` | — |
| `ui/src/components/RgbCurvesEditor.tsx:253` | hex | `#14532d` | GREEN(144deg) |
| `ui/src/components/RgbCurvesEditor.tsx:253` | hex | `#4c1d3d` | — |
| `ui/src/components/RgbCurvesEditor.tsx:285` | hex | `#462d40` | — |
| `ui/src/components/RgbCurvesEditor.tsx:291` | hex | `#462d40` | — |
| `ui/src/components/RgbCurvesEditor.tsx:297` | hex | `#462d40` | — |
| `ui/src/components/RgbCurvesEditor.tsx:303` | hex | `#462d40` | — |
| `ui/src/components/RgbCurvesEditor.tsx:309` | hex | `#462d40` | — |
| `ui/src/components/SourceAudioVisualizer.tsx:103` | hex | `#be185d` | — |
| `ui/src/components/SourceAudioVisualizer.tsx:104` | hex | `#be185d` | — |
| `ui/src/components/SourceAudioVisualizer.tsx:105` | rgba | `rgba(159, 18, 73, 0.35)` | — |
| `ui/src/components/SourceAudioVisualizer.tsx:106` | rgba | `rgba(159, 18, 73, 0.35)` | — |
| `ui/src/components/StatusBar.tsx:84` | hex | `#f472b6` | — |
| `ui/src/components/StatusBar.tsx:123` | hex | `#10b981` | retired, GREEN(160deg) |
| `ui/src/components/StatusBar.tsx:125` | hex | `#f59e0b` | retired |
| `ui/src/components/VideoScopes.tsx:53` | hex | `#050505` | retired |
| `ui/src/components/VideoScopes.tsx:217` | rgba | `rgba(100, 116, 139, 0.45)` | — |
| `ui/src/components/VideoScopes.tsx:217` | rgba | `rgba(71, 85, 105, 0.22)` | — |
| `ui/src/components/VideoScopes.tsx:225` | hex | `#d9468f` | — |
| `ui/src/components/VideoScopes.tsx:225` | hex | `#94a3b8` | — |
| `ui/src/components/VideoScopes.tsx:225` | rgba | `rgba(148, 163, 184, 0.6)` | — |
| `ui/src/components/VideoScopes.tsx:237` | rgba | `rgba(56, 189, 248, ${baseAlpha.toFixed(3)` | — |
| `ui/src/components/VideoScopes.tsx:255` | rgba | `rgba(148, 163, 184, 0.4)` | — |
| `ui/src/components/VideoScopes.tsx:284` | rgba | `rgba(71, 85, 105, 0.35)` | — |
| `ui/src/components/VideoScopes.tsx:294` | rgba | `rgba(100, 116, 139, 0.4)` | — |
| `ui/src/components/VideoScopes.tsx:299` | rgba | `rgba(71, 85, 105, 0.3)` | — |
| `ui/src/components/VideoScopes.tsx:310` | rgba | `rgba(251, 146, 60, 0.45)` | — |
| `ui/src/components/VideoScopes.tsx:317` | hex | `#fb923c` | — |
| `ui/src/components/VideoScopes.tsx:323` | hex | `#ef4444` | — |
| `ui/src/components/VideoScopes.tsx:324` | hex | `#ec4899` | retired |
| `ui/src/components/VideoScopes.tsx:325` | hex | `#3b82f6` | — |
| `ui/src/components/VideoScopes.tsx:326` | hex | `#06b6d4` | — |
| `ui/src/components/VideoScopes.tsx:327` | hex | `#22c55e` | retired, GREEN(142deg) |
| `ui/src/components/VideoScopes.tsx:328` | hex | `#eab308` | retired |
| `ui/src/components/VideoScopes.tsx:349` | rgba | `rgba(245, 158, 11, ${baseAlpha.toFixed(3)` | — |
| `ui/src/components/VideoScopes.tsx:367` | rgba | `rgba(148, 163, 184, 0.4)` | — |
| `ui/src/components/VideoScopes.tsx:394` | hex | `#ef4444` | — |
| `ui/src/components/VideoScopes.tsx:395` | hex | `#22c55e` | retired, GREEN(142deg) |
| `ui/src/components/VideoScopes.tsx:396` | hex | `#3b82f6` | — |
| `ui/src/components/VideoScopes.tsx:405` | rgba | `rgba(71, 85, 105, 0.25)` | — |
| `ui/src/components/VideoScopes.tsx:489` | rgba | `rgba(71, 85, 105, 0.25)` | — |
| `ui/src/components/VideoScopes.tsx:493` | rgba | `rgba(148, 163, 184, 0.5)` | — |
| `ui/src/components/VideoScopes.tsx:524` | rgba | `rgba(148, 163, 184, 0.12)` | — |
| `ui/src/components/VideoScopes.tsx:532` | rgba | `rgba(226, 232, 240, 0.8)` | — |
| `ui/src/components/VideoScopes.tsx:533` | hex | `#ef4444` | — |
| `ui/src/components/VideoScopes.tsx:534` | hex | `#22c55e` | retired, GREEN(142deg) |
| `ui/src/components/VideoScopes.tsx:535` | hex | `#3b82f6` | — |
| `ui/src/components/VideoScopes.tsx:553` | rgba | `rgba(100, 116, 139, 0.3)` | — |
| `ui/src/components/linear/TitleBar.tsx:112` | hex | `#050505` | retired |
| `ui/src/components/linear/TitleBar.tsx:112` | hex | `#F3F4F6` | — |
| `ui/src/components/linear/echarts-linear-theme.ts:5` | hex | `#3355FF` | retired |
| `ui/src/components/linear/echarts-linear-theme.ts:6` | hex | `#38B6F0` | retired |
| `ui/src/components/linear/echarts-linear-theme.ts:7` | hex | `#22C55E` | retired, GREEN(142deg) |
| `ui/src/components/linear/echarts-linear-theme.ts:8` | hex | `#F43F5E` | retired |
| `ui/src/components/linear/echarts-linear-theme.ts:9` | hex | `#EAB308` | retired |
| `ui/src/components/linear/echarts-linear-theme.ts:10` | hex | `#1FB5A5` | retired |
| `ui/src/components/linear/echarts-linear-theme.ts:11` | hex | `#EC4899` | retired |
| `ui/src/components/linear/echarts-linear-theme.ts:12` | hex | `#7C5CE6` | retired |
| `ui/src/components/linear/echarts-linear-theme.ts:13` | hex | `#F97316` | — |
| `ui/src/components/linear/echarts-linear-theme.ts:14` | hex | `#202128` | retired |
| `ui/src/components/linear/echarts-linear-theme.ts:15` | hex | `#EAF0FF` | retired |
| `ui/src/components/linear/echarts-linear-theme.ts:16` | hex | `#1F2125` | retired |
| `ui/src/components/linear/echarts-linear-theme.ts:17` | hex | `#E5E7EB` | — |
| `ui/src/components/linear/echarts-linear-theme.ts:18` | hex | `#8A8B91` | retired |
| `ui/src/components/linear/echarts-linear-theme.ts:19` | hex | `#6B7280` | retired |
| `ui/src/components/linear/echarts-linear-theme.ts:20` | hex | `#111213` | retired |
| `ui/src/components/linear/echarts-linear-theme.ts:21` | hex | `#FFFFFF` | — |
| `ui/src/components/linear/echarts-linear-theme.ts:22` | hex | `#F5F5F6` | retired |
| `ui/src/components/linear/echarts-linear-theme.ts:23` | hex | `#0F1115` | retired |
| `ui/src/components/linear/echarts-linear-theme.ts:63` | rgb | `rgb(16 24 40 / 5%)` | — |
| `ui/src/features/agent/AgentCard.tsx:97` | hex | `#f59e0b` | retired |
| `ui/src/features/agent/AgentCard.tsx:99` | hex | `#f472b6` | — |
| `ui/src/features/agent/AgentCard.tsx:100` | hex | `#9ca3af` | retired |
| `ui/src/features/agent/AgentCard.tsx:114` | hex | `#9ca3af` | retired |
| `ui/src/features/agent/AgentCard.tsx:175` | hex | `#10b981` | retired, GREEN(160deg) |
| `ui/src/features/agent/AgentCard.tsx:177` | hex | `#f59e0b` | retired |
| `ui/src/features/agent/AgentCard.tsx:238` | hex | `#9ca3af` | retired |
| `ui/src/features/agent/AgentCard.tsx:314` | hex | `#9ca3af` | retired |
| `ui/src/features/agent/AgentCard.tsx:322` | hex | `#9ca3af` | retired |
| `ui/src/features/agent/AgentCard.tsx:331` | hex | `#9ca3af` | retired |
| `ui/src/features/agent/AgentCard.tsx:339` | hex | `#9ca3af` | retired |
| `ui/src/features/agent/AgentCard.tsx:348` | hex | `#9ca3af` | retired |
| `ui/src/features/agent/AgentCard.tsx:366` | hex | `#d1d5db` | — |
| `ui/src/features/agent/AgentCard.tsx:434` | hex | `#9ca3af` | retired |
| `ui/src/features/agent/AgentCard.tsx:473` | hex | `#9ca3af` | retired |
| `ui/src/features/agent/AgentCard.tsx:479` | hex | `#9ca3af` | retired |
| `ui/src/features/agent/AgentStatusChip.tsx:25` | hex | `#f59e0b` | retired |
| `ui/src/features/agent/AgentStatusChip.tsx:27` | hex | `#f472b6` | — |
| `ui/src/features/agent/AgentStatusChip.tsx:29` | hex | `#c084fc` | — |
| `ui/src/features/agent/AgentStatusChip.tsx:30` | hex | `#9ca3af` | retired |
| `ui/src/features/agent/AgentStatusChip.tsx:74` | hex | `#9ca3af` | retired |
| `ui/src/features/agent/ReviewCard.tsx:125` | hex | `#f59e0b` | retired |
| `ui/src/features/agent/ReviewCard.tsx:157` | hex | `#9ca3af` | retired |
| `ui/src/features/agent/ReviewCard.tsx:238` | hex | `#9ca3af` | retired |
| `ui/src/features/agent/ReviewCard.tsx:258` | hex | `#d1d5db` | — |
| `ui/src/features/agent/ReviewCard.tsx:344` | hex | `#d1d5db` | — |
| `ui/src/features/agent/ReviewCard.tsx:370` | hex | `#9ca3af` | retired |
| `ui/src/features/captions/CaptionsPanel.tsx:204` | hex | `#ffffff` | — |
| `ui/src/features/captions/CaptionsPanel.tsx:205` | hex | `#000000` | — |
| `ui/src/features/captions/CaptionsPanel.tsx:207` | hex | `#000000` | — |
| `ui/src/features/dialogs/GpuTelemetryModal.tsx:44` | hex | `#110910` | — |
| `ui/src/features/dialogs/GpuTelemetryModal.tsx:44` | hex | `#221220` | — |
| `ui/src/features/dialogs/GpuTelemetryModal.tsx:45` | hex | `#7d6377` | — |
| `ui/src/features/dialogs/GpuTelemetryModal.tsx:46` | hex | `#be185d` | — |
| `ui/src/features/dialogs/GpuTelemetryModal.tsx:47` | hex | `#10b981` | retired, GREEN(160deg) |
| `ui/src/features/dialogs/GpuTelemetryModal.tsx:49` | hex | `#110910` | — |
| `ui/src/features/dialogs/GpuTelemetryModal.tsx:49` | hex | `#221220` | — |
| `ui/src/features/dialogs/GpuTelemetryModal.tsx:50` | hex | `#7d6377` | — |
| `ui/src/features/dialogs/GpuTelemetryModal.tsx:51` | hex | `#e2e8f0` | — |
| `ui/src/features/dialogs/GpuTelemetryModal.tsx:52` | hex | `#bfa5b8` | — |
| `ui/src/features/dialogs/GpuTelemetryModal.tsx:54` | hex | `#110910` | — |
| `ui/src/features/dialogs/GpuTelemetryModal.tsx:54` | hex | `#221220` | — |
| `ui/src/features/dialogs/GpuTelemetryModal.tsx:55` | hex | `#7d6377` | — |
| `ui/src/features/dialogs/GpuTelemetryModal.tsx:56` | hex | `#e2e8f0` | — |
| `ui/src/features/dialogs/GpuTelemetryModal.tsx:57` | hex | `#bfa5b8` | — |
| `ui/src/features/dialogs/GpuTelemetryModal.tsx:59` | hex | `#110910` | — |
| `ui/src/features/dialogs/GpuTelemetryModal.tsx:59` | hex | `#221220` | — |
| `ui/src/features/dialogs/GpuTelemetryModal.tsx:60` | hex | `#7d6377` | — |
| `ui/src/features/dialogs/GpuTelemetryModal.tsx:61` | hex | `#e2e8f0` | — |
| `ui/src/features/dialogs/GpuTelemetryModal.tsx:62` | hex | `#bfa5b8` | — |
| `ui/src/features/dialogs/GpuTelemetryModal.tsx:64` | hex | `#110910` | — |
| `ui/src/features/dialogs/GpuTelemetryModal.tsx:64` | hex | `#221220` | — |
| `ui/src/features/dialogs/GpuTelemetryModal.tsx:65` | hex | `#7d6377` | — |
| `ui/src/features/dialogs/GpuTelemetryModal.tsx:66` | hex | `#e2e8f0` | — |
| `ui/src/features/dialogs/GpuTelemetryModal.tsx:67` | hex | `#bfa5b8` | — |
| `ui/src/features/dialogs/GpuTelemetryModal.tsx:69` | hex | `#110910` | — |
| `ui/src/features/dialogs/GpuTelemetryModal.tsx:69` | hex | `#221220` | — |
| `ui/src/features/dialogs/GpuTelemetryModal.tsx:70` | hex | `#7d6377` | — |
| `ui/src/features/dialogs/GpuTelemetryModal.tsx:71` | hex | `#10b981` | retired, GREEN(160deg) |
| `ui/src/features/dialogs/GpuTelemetryModal.tsx:72` | hex | `#10b981` | retired, GREEN(160deg) |
| `ui/src/features/dialogs/InteropModal.tsx:79` | hex | `#110910` | — |
| `ui/src/features/dialogs/InteropModal.tsx:80` | hex | `#462d40` | — |
| `ui/src/features/dialogs/InteropModal.tsx:82` | hex | `#e2e8f0` | — |
| `ui/src/features/dialogs/InteropModal.tsx:126` | hex | `#110910` | — |
| `ui/src/features/dialogs/InteropModal.tsx:127` | hex | `#462d40` | — |
| `ui/src/features/dialogs/InteropModal.tsx:129` | hex | `#e2e8f0` | — |
| `ui/src/features/dialogs/MediaBinModal.tsx:55` | hex | `#be185d` | — |
| `ui/src/features/dialogs/MediaBinModal.tsx:55` | hex | `#10b981` | retired, GREEN(160deg) |
| `ui/src/features/dialogs/MediaBinModal.tsx:55` | hex | `#f59e0b` | retired |
| `ui/src/features/dialogs/MediaBinModal.tsx:55` | hex | `#ef4444` | — |
| `ui/src/features/dialogs/MediaBinModal.tsx:55` | hex | `#6b21a8` | — |
| `ui/src/features/dialogs/MediaBinModal.tsx:55` | hex | `#ec4899` | retired |
| `ui/src/features/dialogs/MediaBinModal.tsx:55` | hex | `#7d6377` | — |
| `ui/src/features/dialogs/MediaBinModal.tsx:64` | hex | `#ffffff` | — |
| `ui/src/features/dialogs/MediaPropsModal.tsx:116` | hex | `#be185d` | — |
| `ui/src/features/dialogs/MediaPropsModal.tsx:116` | hex | `#10b981` | retired, GREEN(160deg) |
| `ui/src/features/dialogs/MediaPropsModal.tsx:116` | hex | `#f59e0b` | retired |
| `ui/src/features/dialogs/MediaPropsModal.tsx:116` | hex | `#ef4444` | — |
| `ui/src/features/dialogs/MediaPropsModal.tsx:116` | hex | `#6b21a8` | — |
| `ui/src/features/dialogs/MediaPropsModal.tsx:116` | hex | `#ec4899` | retired |
| `ui/src/features/dialogs/MediaPropsModal.tsx:116` | hex | `#7d6377` | — |
| `ui/src/features/dialogs/MediaPropsModal.tsx:125` | hex | `#ffffff` | — |
| `ui/src/features/dialogs/MediaPropsModal.tsx:125` | hex | `#7d6377` | — |
| `ui/src/features/dialogs/MediaPropsModal.tsx:132` | hex | `#bfa5b8` | — |
| `ui/src/features/graphics/CanvasGraphicsLayer.tsx:288` | hex | `#ffffff` | — |
| `ui/src/features/graphics/GraphicsPanel.tsx:321` | hex | `#ffffff` | — |
| `ui/src/features/graphics/GraphicsPanel.tsx:403` | hex | `#ffffff` | — |
| `ui/src/features/graphics/graphicsOverlay.ts:88` | hex | `#ffffff` | — |
| `ui/src/features/graphics/graphicsOverlay.ts:96` | hex | `#ffffff` | — |
| `ui/src/features/graphics/graphicsOverlay.ts:109` | hex | `#000000` | — |
| `ui/src/features/graphics/graphicsOverlay.ts:193` | rgba | `rgba(${r}, ${g}, ${b}, ${clamp(opacity, 0, 1)` | — |
| `ui/src/features/inspector/AgentTelemetryDrawer.tsx:217` | hex | `#10b981` | retired, GREEN(160deg) |
| `ui/src/features/inspector/AgentTelemetryDrawer.tsx:217` | hex | `#f59e0b` | retired |
| `ui/src/features/inspector/AgentTelemetryDrawer.tsx:234` | hex | `#f59e0b` | retired |
| `ui/src/features/inspector/AgentTelemetryDrawer.tsx:236` | hex | `#f472b6` | — |
| `ui/src/features/inspector/AgentTelemetryDrawer.tsx:237` | hex | `#38bdf8` | — |
| `ui/src/features/inspector/EffectsInspector.tsx:343` | hex | `#00ff00` | GREEN(120deg) |
| `ui/src/features/inspector/TitleInspector.tsx:34` | hex | `#ffffff` | — |
| `ui/src/features/inspector/TitleInspector.tsx:35` | rgba | `rgba(15, 23, 42, 0.75)` | — |
| `ui/src/features/inspector/TitleInspector.tsx:87` | hex | `#ffffff` | — |
| `ui/src/features/preferences/useAppPreferences.ts:35` | hex | `#050505` | retired |
| `ui/src/features/preferences/useAppPreferences.ts:35` | hex | `#F3F4F6` | — |
| `ui/src/features/preview/PreviewSection.tsx:278` | rgba | `rgba(15, 23, 42, 0.75)` | — |
| `ui/src/features/preview/PreviewSection.tsx:341` | rgba | `rgba(0,0,0,${deckSnapshot.deckA.vignette.intensity ?? 0.5})` | — |
| `ui/src/features/preview/PreviewSection.tsx:362` | rgba | `rgba(0,0,0,${deckSnapshot.deckB.vignette.intensity ?? 0.5})` | — |
| `ui/src/features/tasksheet/AgentInbox.tsx:70` | hex | `#f59e0b` | retired |
| `ui/src/features/tasksheet/AgentInbox.tsx:87` | hex | `#f59e0b` | retired |
| `ui/src/features/tasksheet/AgentInbox.tsx:160` | hex | `#10b981` | retired, GREEN(160deg) |
| `ui/src/features/tasksheet/AgentInbox.tsx:161` | hex | `#10b981` | retired, GREEN(160deg) |
| `ui/src/features/tasksheet/TaskSheetPanel.tsx:144` | hex | `#10b981` | retired, GREEN(160deg) |
| `ui/src/features/tasksheet/TaskSheetPanel.tsx:147` | hex | `#f472b6` | — |
| `ui/src/features/tasksheet/TaskSheetPanel.tsx:151` | hex | `#f59e0b` | retired |
| `ui/src/features/tasksheet/TaskSheetPanel.tsx:154` | hex | `#ef4444` | — |
| `ui/src/features/tasksheet/TaskSheetPanel.tsx:156` | hex | `#9ca3af` | retired |
| `ui/src/features/tasksheet/TaskSheetPanel.tsx:351` | hex | `#f59e0b` | retired |
| `ui/src/features/tasksheet/TaskSheetPanel.tsx:435` | hex | `#f59e0b` | retired |
| `ui/src/features/tasksheet/TaskSheetPanel.tsx:438` | hex | `#f59e0b` | retired |
| `ui/src/features/tasksheet/TaskSheetPanel.tsx:447` | hex | `#10b981` | retired, GREEN(160deg) |
| `ui/src/features/tasksheet/TaskSheetPanel.tsx:450` | hex | `#10b981` | retired, GREEN(160deg) |
| `ui/src/features/tasksheet/TaskSheetPanel.tsx:489` | hex | `#10b981` | retired, GREEN(160deg) |
| `ui/src/features/timeline/AiBranchDiffRibbon.tsx:104` | hex | `#f59e0b` | retired |
| `ui/src/features/timeline/AiBranchDiffRibbon.tsx:106` | hex | `#38bdf8` | — |
| `ui/src/features/timeline/AiBranchDiffRibbon.tsx:107` | hex | `#10b981` | retired, GREEN(160deg) |
| `ui/src/features/timeline/AiBranchDiffRibbon.tsx:126` | hex | `#ffffff` | — |
| `ui/src/features/timeline/TimelineTrackLane.tsx:71` | hex | `#9f1249` | — |
| `ui/src/features/timeline/TimelineTrackLane.tsx:197` | hex | `#9f1249` | — |
| `ui/src/features/timeline/TimelineTrackLane.tsx:333` | hex | `#34d399` | GREEN(158deg) |
| `ui/src/features/timeline/TimelineTrackLane.tsx:333` | hex | `#be185d` | — |
| `ui/src/features/timeline/TimelineTrackLane.tsx:334` | rgba | `rgba(52, 211, 153, 0.42)` | GREEN(158deg) |
| `ui/src/features/timeline/TimelineTrackLane.tsx:334` | rgba | `rgba(159, 18, 73, 0.42)` | — |
| `ui/src/features/timeline/labelColors.ts:12` | hex | `#3b82f6` | — |
| `ui/src/features/timeline/labelColors.ts:13` | hex | `#a16207` | — |
| `ui/src/features/timeline/labelColors.ts:14` | hex | `#0d9488` | retired |
| `ui/src/features/timeline/labelColors.ts:15` | hex | `#0284c7` | — |
| `ui/src/features/timeline/labelColors.ts:16` | hex | `#166534` | GREEN(143deg) |
| `ui/src/features/timeline/labelColors.ts:17` | hex | `#22c55e` | retired, GREEN(142deg) |
| `ui/src/features/timeline/labelColors.ts:18` | hex | `#6366f1` | — |
| `ui/src/features/timeline/labelColors.ts:19` | hex | `#c084fc` | — |
| `ui/src/features/timeline/labelColors.ts:20` | hex | `#db2777` | — |
| `ui/src/features/timeline/labelColors.ts:21` | hex | `#f59e0b` | retired |
| `ui/src/features/timeline/labelColors.ts:22` | hex | `#9333ea` | — |
| `ui/src/features/timeline/labelColors.ts:23` | hex | `#f43f5e` | retired |
| `ui/src/features/timeline/labelColors.ts:24` | hex | `#d6bd8f` | — |
| `ui/src/features/timeline/labelColors.ts:25` | hex | `#14b8a6` | — |
| `ui/src/features/timeline/labelColors.ts:26` | hex | `#8b5cf6` | — |
| `ui/src/features/timeline/labelColors.ts:27` | hex | `#facc15` | — |
| `ui/src/styles/00-tokens-glass.css:17` | hex | `#A78BFA` | — |
| `ui/src/styles/01-editor.css:27` | hex | `#FFFFFF` | — |
| `ui/src/styles/01-editor.css:28` | hex | `#FFFFFF` | — |
| `ui/src/styles/01-editor.css:99` | hex | `#FFFFFF` | — |
| `ui/src/styles/01-editor.css:100` | hex | `#FFFFFF` | — |
| `ui/src/styles/01-editor.css:151` | hex | `#050505` | retired |
| `ui/src/styles/01-editor.css:152` | hex | `#050505` | retired |
| `ui/src/styles/01-editor.css:153` | hex | `#111213` | retired |
| `ui/src/styles/01-editor.css:154` | hex | `#111213` | retired |
| `ui/src/styles/01-editor.css:155` | hex | `#1A1B1E` | retired |
| `ui/src/styles/01-editor.css:156` | hex | `#1A1B1E` | retired |
| `ui/src/styles/01-editor.css:157` | hex | `#202128` | retired |
| `ui/src/styles/01-editor.css:158` | hex | `#2C2E33` | retired |
| `ui/src/styles/01-editor.css:159` | hex | `#2C2E33` | retired |
| `ui/src/styles/01-editor.css:160` | hex | `#5A5B62` | retired |
| `ui/src/styles/01-editor.css:161` | hex | `#5A5B62` | retired |
| `ui/src/styles/01-editor.css:162` | hex | `#8A8B91` | retired |
| `ui/src/styles/01-editor.css:163` | hex | `#8A8B91` | retired |
| `ui/src/styles/01-editor.css:164` | hex | `#D1D5DB` | — |
| `ui/src/styles/01-editor.css:165` | hex | `#E5E7EB` | — |
| `ui/src/styles/01-editor.css:166` | hex | `#F3F4F6` | — |
| `ui/src/styles/01-editor.css:167` | hex | `#8A8B91` | retired |
| `ui/src/workspaces/docks/EffectsDocks.tsx:307` | hex | `#00ff00` | GREEN(120deg) |
| `ui/src/workspaces/docks/EffectsDocks.tsx:313` | hex | `#00ff00` | GREEN(120deg) |
| `ui/src/workspaces/docks/FusionDocks.tsx:136` | hex | `#00ff00` | GREEN(120deg) |
| `ui/src/workspaces/docks/FusionDocks.tsx:176` | hex | `#be185d` | — |
| `ui/src/workspaces/docks/FusionDocks.tsx:177` | hex | `#a855f7` | retired |
| `ui/src/workspaces/docks/FusionDocks.tsx:180` | hex | `#f59e0b` | retired |
| `ui/src/workspaces/docks/FusionDocks.tsx:181` | hex | `#ec4899` | retired |
| `ui/src/workspaces/docks/FusionDocks.tsx:350` | hex | `#00ff00` | GREEN(120deg) |
| `ui/src/workspaces/fairlight/DspRackPanel.tsx:103` | hex | `#6b21a8` | — |
| `ui/src/workspaces/fairlight/DspRackPanel.tsx:104` | hex | `#6b21a8` | — |
| `ui/src/workspaces/fairlight/DspRackPanel.tsx:107` | hex | `#462d40` | — |
| `ui/src/workspaces/fairlight/DspRackPanel.tsx:109` | hex | `#d9468f` | — |
| `ui/src/workspaces/fairlight/DspRackPanel.tsx:109` | hex | `#7d6377` | — |
| `ui/src/workspaces/fairlight/DspRackPanel.tsx:110` | hex | `#be185d` | — |
| `ui/src/workspaces/fairlight/DspRackPanel.tsx:111` | hex | `#a78bfa` | — |
| `ui/src/workspaces/fairlight/DspRackPanel.tsx:112` | hex | `#f59e0b` | retired |
| `ui/src/workspaces/fairlight/DspRackPanel.tsx:113` | hex | `#34d399` | GREEN(158deg) |
