// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
export function buildMediaStreamUrl(
  filePath: string,
  mediaServerPort: number,
  mediaServerToken?: string
): string {
  if (!filePath || mediaServerPort <= 0) return "";
  const tokenPart = mediaServerToken ? `&token=${encodeURIComponent(mediaServerToken)}` : "";
  return `http://127.0.0.1:${mediaServerPort}/media?path=${encodeURIComponent(filePath)}${tokenPart}`;
}
