// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
import {
  Brain, Camera, Circle, Clapperboard, CloudSun, Diamond, Droplets, Film, Flag,
  Flame, Folder, FolderOpen, Globe, Hourglass, Leaf, Monitor, Orbit, Palette,
  Ruler, Scissors, Shuffle, SlidersHorizontal, Smartphone, Sparkle, Sparkles,
  Square, Sunrise, Target, Tv, Video, WandSparkles, Zap,
  type LucideIcon,
} from "lucide-react";

/** Named line icons for data records that used to carry emoji in an `icon` field. */
const glyphs: Record<string, LucideIcon> = {
  Brain, Camera, Circle, Clapperboard, CloudSun, Diamond, Droplets, Film, Flag,
  Flame, Folder, FolderOpen, Globe, Hourglass, Leaf, Monitor, Orbit, Palette,
  Ruler, Scissors, Shuffle, SlidersHorizontal, Smartphone, Sparkle, Sparkles,
  Square, Sunrise, Target, Tv, Video, WandSparkles, Zap,
};

export function LucideGlyph({
  name,
  size = 16,
  className,
}: {
  name: string;
  size?: number;
  className?: string;
}) {
  const Glyph = glyphs[name] ?? Circle;
  return <Glyph size={size} className={className} aria-hidden="true" strokeWidth={1.5} />;
}
