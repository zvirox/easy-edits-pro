#!/usr/bin/env bash

# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
# Also available under the EEP commercial licence — see LICENSING.md
# Creates or updates the standard issue labels with the GitHub CLI.
# Usage: gh auth login && scripts/github-labels.sh [owner/repo]
set -euo pipefail
repo="${1:-zvirox/easy-edits-pro}"
mk() { gh label create "$1" --repo "$repo" --color "$2" --description "$3" --force; }
mk "good first issue"      7057ff "Small, well-specified task for newcomers"
mk "help wanted"           008672 "Maintainers would welcome a contribution"
mk "needs design"          d4c5f9 "Needs a design decision before work starts"
mk "needs repro"           fbca04 "Cannot reproduce yet; more information needed"
mk "blocked"               b60205 "Waiting on something else"
mk "question"              d876e3 "Further information is requested"
mk "difficulty: easy"      c2e0c6 "A few hours"
mk "difficulty: medium"    fef2c0 "A day or two"
mk "difficulty: hard"      f9d0c4 "Needs deep knowledge of the area"
for a in ui engine render audio agent docs packaging; do
  mk "area: $a" 0e8a16 "Area: $a"
done
echo "Labels updated on $repo"
