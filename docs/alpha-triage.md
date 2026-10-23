# Alpha triage playbook (M2.3)

How incoming alpha/beta issue reports are turned into well-scoped fixes
without inventing hardware-compatibility claims. The category dropdown in the
bug-report template is the entry point; this file is the routing table and
cadence.

## Categories and first checks

| Category | First checks on a report |
|---|---|
| Import | attached media codec (ffprobe), import notice text (should name the reason), diagnostics package set |
| Preview | missing-media badge state, proxy job state, diagnostics GPU row |
| Timeline | reproduce with clean project; check undo/redo path; whether save/reopen preserves the corruption |
| Audio | sample rate/device, automation mode, mixer meters |
| Export / Render | job state in the export queue, partial output on cancel, codec preset vs error text |
| MCP | bridge socket present, desktop-app version match, tool error payload |
| Package installation / launch | distro, install method, launch from terminal for stderr, diagnostics if it launches |
| UI / Other | screenshot, minimal repro steps |

## Cadence

- **Triage pass: every 2-3 days.** Label each new issue with its area
  (`area/import`, `area/timeline`, ...), `alpha-feedback`, and
  `reproducible-laptop` when reproduced here.
- **Reproduction attempt: same pass.** Anything reproducible on this laptop
  gets a regression test (engine test, UI test, or native workflow step)
  before or with the fix - a fix without a test does not close the issue.
- **Fix train: every alpha.** Reproduced-and-fixed reports are published in
  the next alpha release with a note pointing at the issue.
- **Cannot reproduce:** ask for diagnostics + codec + version, mark
  `needs-info`; close after two unanswered follow-ups (recorded in the
  issue, not deleted).

## Hard rules

1. Reproduce on this laptop where possible; feedback that cannot be
   reproduced here is prioritised by how many reports share the same area,
   never by assumption about hardware we cannot test.
2. Never record package install/launch compatibility from build success.
   Only supplied real results enter [distribution.md](distribution.md).
3. Reported data-loss or unrecoverable-edit regressions block the next alpha
   (beta candidacy requires several alphas without one).

## Closing loop

Every closed alpha issue should state: root cause, the regression that now
guards it, and the release that carried the fix.
