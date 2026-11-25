// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
export const LINEAR_COLORS = {
  blue: '#3355FF',
  sky: '#38B6F0',
  green: '#22C55E',
  red: '#F43F5E',
  amber: '#EAB308',
  teal: '#1FB5A5',
  pink: '#EC4899',
  purple: '#7C5CE6',
  orange: '#F97316',
  idleDark: '#202128',
  idleLight: '#EAF0FF',
  borderDark: '#1F2125',
  borderLight: '#E5E7EB',
  textDark: '#8A8B91',
  textLight: '#6B7280',
  surfaceDark: '#111213',
  surfaceLight: '#FFFFFF',
  fgDark: '#F5F5F6',
  fgLight: '#0F1115',
};

export const LINEAR_SERIES_PALETTE = [
  LINEAR_COLORS.blue,
  LINEAR_COLORS.sky,
  LINEAR_COLORS.green,
  LINEAR_COLORS.amber,
  LINEAR_COLORS.teal,
  LINEAR_COLORS.pink,
  LINEAR_COLORS.purple,
  LINEAR_COLORS.orange,
] as const;

export const getLinearEChartsOption = (isDark: boolean = true) => ({
  backgroundColor: 'transparent',
  textStyle: {
    fontFamily: '"Inter Variable", "Inter", system-ui, -apple-system, sans-serif',
    fontSize: 12,
    color: isDark ? LINEAR_COLORS.textDark : LINEAR_COLORS.textLight,
  },
  grid: {
    top: 24,
    right: 16,
    bottom: 24,
    left: 48,
    borderColor: isDark ? LINEAR_COLORS.borderDark : LINEAR_COLORS.borderLight,
  },
  tooltip: {
    backgroundColor: isDark ? LINEAR_COLORS.surfaceDark : LINEAR_COLORS.surfaceLight,
    borderColor: isDark ? LINEAR_COLORS.borderDark : LINEAR_COLORS.borderLight,
    borderWidth: 1,
    padding: [8, 12],
    textStyle: {
      color: isDark ? LINEAR_COLORS.fgDark : LINEAR_COLORS.fgLight,
      fontSize: 12,
      fontFamily: '"Inter Variable", "Inter", system-ui, -apple-system, sans-serif',
    },
    extraCssText: isDark
      ? 'border-radius: 6px; box-shadow: none;'
      : 'border-radius: 6px; box-shadow: 0 1px 2px rgb(16 24 40 / 5%);',
  },
  xAxis: {
    axisLine: { lineStyle: { color: isDark ? LINEAR_COLORS.borderDark : LINEAR_COLORS.borderLight } },
    axisLabel: { fontSize: 12, color: isDark ? LINEAR_COLORS.textDark : LINEAR_COLORS.textLight },
    splitLine: { show: false },
    axisTick: { show: false },
  },
  yAxis: {
    axisLine: { show: false },
    axisLabel: { fontSize: 12, color: isDark ? LINEAR_COLORS.textDark : LINEAR_COLORS.textLight },
    splitLine: {
      lineStyle: {
        type: 'dotted' as const,
        color: isDark ? LINEAR_COLORS.borderDark : LINEAR_COLORS.borderLight,
      },
    },
  },
  color: [...LINEAR_SERIES_PALETTE],
});

/** Helper for gradient fade under sparkline and line series (Section 1 exception). */
export const getLinearAreaGradient = (color: string) => ({
  type: 'linear' as const,
  x: 0,
  y: 0,
  x2: 0,
  y2: 1,
  colorStops: [
    { offset: 0, color },
    { offset: 1, color: 'transparent' },
  ],
});
