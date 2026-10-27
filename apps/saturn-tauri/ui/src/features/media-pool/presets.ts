import type { TransitionKind } from "../../app/projectTypes";

export const titlePresets = [
  {
    id: "lower_third",
    name: "Broadcast Lower-Third",
    desc: "Presenter name & role banner",
    preview: "John Doe · Lead Editor",
    style: "lower_third",
    text: "Presenter Name",
    subtitle: "Lead Video Editor",
  },
  {
    id: "minimal_pill",
    name: "Minimal Pill",
    desc: "Modern frosted floating badge",
    preview: "✦ MINIMAL PILL",
    style: "minimal_pill",
    text: "Minimal Pill Headline",
    subtitle: "Subtle contextual note",
  },
  {
    id: "center_bold",
    name: "Cinematic Headline",
    desc: "Bold high-impact centered title",
    preview: "EPISODE 01",
    style: "center_bold",
    text: "CINEMATIC HEADLINE",
    subtitle: "A NEW CHAPTER BEGINS",
  },
  {
    id: "cyber_neon",
    name: "Cyber Neon",
    desc: "Vibrant glowing title effect",
    preview: "⚡ CYBER NEON",
    style: "cyber_neon",
    text: "CYBER NEON 2026",
    subtitle: "NIGHT CITY PROTOCOL",
  },
  {
    id: "headline_subtitle",
    name: "Documentary Card",
    desc: "Headline with location subtext",
    preview: "DOCUMENTARY",
    style: "headline_subtitle",
    text: "Documentary Chapter",
    subtitle: "Location and historical context",
  },
  {
    id: "boxed_banner",
    name: "Framed Banner",
    desc: "Aesthetic boxed caption overlay",
    preview: "[ FRAMED BOX ]",
    style: "boxed_banner",
    text: "Framed Aesthetic Banner",
    subtitle: "Curated sequence notes",
  },
];

export const transitionPresets: { id: TransitionKind; name: string; desc: string; iconSymbol: string }[] = [
  { id: "crossfade", name: "Crossfade", desc: "Smooth blend dissolve between clips", iconSymbol: "⧓" },
  { id: "dip_to_black", name: "Dip to Black", desc: "Cinematic fade through black", iconSymbol: "◼" },
  { id: "dip_to_white", name: "Dip to White", desc: "High-energy flash through white", iconSymbol: "◻" },
  { id: "wipe_left", name: "Wipe Left", desc: "Horizontal wipe moving left", iconSymbol: "◀|" },
  { id: "wipe_right", name: "Wipe Right", desc: "Horizontal wipe moving right", iconSymbol: "|▶" },
  { id: "slide_left", name: "Slide Left", desc: "Push slide moving left", iconSymbol: "⇠" },
  { id: "slide_right", name: "Slide Right", desc: "Push slide moving right", iconSymbol: "⇢" },
];
