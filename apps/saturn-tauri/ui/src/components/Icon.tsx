import React from "react";

export type IconName =
  | "fullscreen"
  | "restore"
  | "close"
  | "media"
  | "add"
  | "undo"
  | "redo"
  | "select"
  | "trackForward"
  | "ripple"
  | "rolling"
  | "razor"
  | "slip"
  | "slide"
  | "rateStretch"
  | "link"
  | "unlink"
  | "pen"
  | "hand"
  | "snap"
  | "split"
  | "remove"
  | "markIn"
  | "markOut"
  | "zoomIn"
  | "zoomOut"
  | "clearMarks"
  | "export"
  | "fileNew"
  | "fileOpen"
  | "fileSave"
  | "import"
  | "help"
  | "window"
  | "title"
  | "transition"
  | "sun"
  | "moon"
  | "color"
  | "audio"
  | "effects"
  | "deliver"
  | "edit"
  | "check"
  | "keyframe"
  | "subtitle";

export function Icon({ name }: { name: IconName }) {
  const paths: Record<IconName, React.ReactNode> = {
    check: <path d="m4 10 4 4 8-8" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />,
    fullscreen: <path d="M7 3H3v4m10-4h4v4M3 13v4h4m10-4v4h-4" />,
    restore: <path d="M7 3h10v10M13 7H3v10h10V7Z" />,
    close: <path d="m5 5 10 10M15 5 5 15" />,
    media: <><rect x="3" y="4" width="18" height="16" rx="2" /><path d="m9 4 2 16m5-16 2 16M3 9h18M3 15h18" /></>,
    add: <path d="M12 5v14m-7-7h14" />,
    undo: <path d="M9 14 4 9l5-5M4 9h9a6 6 0 0 1 0 12h-2" />,
    redo: <path d="m15 14 5-5-5-5m5 5h-9a6 6 0 0 0 0 12h2" />,
    select: <path d="m5 3 10 9-5 .7L8 18z" />,
    trackForward: <><path d="M3 4h8v4H3zM3 10h8v4H3zM3 16h8v2H3z" /><path d="M13 10h5m-2-2 2 2-2 2" /></>,
    ripple: <><path d="m8 5-4 5 4 5M16 5l4 5-4 5M10 10h4" /><path d="M3 18h14" /></>,
    rolling: <><path d="M7 4v12m6-12v12M4 8l3 3-3 3m12-6-3 3 3 3" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" /></>,
    razor: <><circle cx="6" cy="6" r="2" /><circle cx="6" cy="14" r="2" /><path d="m8 7 9 9M8 13l9-9" /></>,
    slip: <><path d="M3 4h14v12H3zM6 2v2m8-2v2m-8 12v2m8-2v2" /><path d="M8 10h6m-2-2 2 2-2 2" /></>,
    slide: <><rect x="6" y="5" width="8" height="10" rx="1" fill="none" stroke="currentColor" strokeWidth="1.5" /><path d="M3 10H1m0 0 1.5-1.5M1 10l1.5 1.5m16.5-1.5h-2m0 0-1.5-1.5m1.5 1.5 1.5 1.5" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" /></>,
    rateStretch: <><circle cx="10" cy="10" r="7" fill="none" stroke="currentColor" strokeWidth="1.5" /><path d="M10 6v4l2.5 1.5M2 10h1.5m13 0h1.5" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" /></>,
    link: <path d="M8 11a3.5 3.5 0 0 0 5 .5l2-2a3.5 3.5 0 0 0-5-5l-1 1M12 9a3.5 3.5 0 0 0-5-.5l-2 2a3.5 3.5 0 0 0 5 5l1-1" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />,
    unlink: <><path d="M13.5 8.5l1.5-1.5a3.5 3.5 0 0 0-5-5l-1 1M6.5 11.5l-1.5 1.5a3.5 3.5 0 0 0 5 5l1-1M3 3l14 14" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" /></>,
    pen: <><path d="m4 14 9-9 3 3-9 9H4zM12 6l3 3M4 17h12" /></>,
    hand: <path d="M6 11V5a1.5 1.5 0 0 1 3 0v5-7a1.5 1.5 0 0 1 3 0v7-5a1.5 1.5 0 0 1 3 0v6-3a1.5 1.5 0 0 1 3 0v5c0 4-2 7-6 7h-1c-2 0-3.5-1-4.5-2.5L3 13a1.8 1.8 0 0 1 3-2z" />,
    snap: <path d="M4 3v5a6 6 0 0 0 12 0V3M4 8h12M10 14v3m-3 0h6" />,
    split: <><path d="M4 4h12v12H4zM10 4v12" /><path d="m7 8 3 2-3 2m6-4-3 2 3 2" /></>,
    remove: <><path d="M4 6h12m-10 0 1 11h6l1-11M8 6V4h4v2m-3 3v5m2-5v5" /></>,
    markIn: <path d="M6 3H4v14h2m10-14h-2m-2 0h-2" />,
    markOut: <path d="M14 3h2v14h-2M4 3h2m2 0h2" />,
    zoomIn: <><circle cx="8.5" cy="8.5" r="5.5" /><path d="m13 13 4 4M8.5 6v5m-2.5-2.5h5" /></>,
    zoomOut: <><circle cx="8.5" cy="8.5" r="5.5" /><path d="m13 13 4 4M6 8.5h5" /></>,
    clearMarks: <><circle cx="10" cy="10" r="7" /><path d="m8 8 4 4m0-4-4 4" /></>,
    export: <><path d="M10 13V3m-4 4 4-4 4 4M4 12v5h12v-5" /></>,
    fileNew: <><path d="M5 2h7l4 4v12H5zM12 2v5h4" /><path d="M10 10v6m-3-3h6" /></>,
    fileOpen: <path d="M2 5h6l2 2h8v10H2zM2 8h16" />,
    fileSave: <><path d="M3 3h12l2 2v12H3zM6 3v5h8V3M6 17v-6h8v6" /><path d="M10 12v4m-2-2h4" /></>,
    import: <><path d="M10 3v10m-4-4 4 4 4-4M3 15v3h14v-3" /></>,
    help: <><circle cx="10" cy="10" r="8" /><path d="M7.8 7.5A2.3 2.3 0 1 1 11 9.7c-.8.5-1 1-1 2m0 2.5v.1" /></>,
    window: <><rect x="2" y="3" width="16" height="14" rx="1" /><path d="M2 7h16M7 7v10" /></>,
    title: <path d="M4 5h12M10 5v11m-3 0h6" />,
    transition: <path d="M3 5h4l6 10h4M3 15h4l6-10h4" />,
    sun: <><circle cx="10" cy="10" r="4" /><path d="M10 2v2m0 12v2M2 10h2m12 0h2m-3.5-4.5 1.4-1.4M4.9 15.1l1.4-1.4m0-7.4L4.9 4.9m10.2 10.2-1.4-1.4" /></>,
    moon: <path d="M16 11.5A6.5 6.5 0 0 1 8.5 4a6.5 6.5 0 1 0 7.5 7.5z" />,
    color: <path d="M10 2a8 8 0 1 0 8 8 3 3 0 0 0-3-3h-1.5a1.5 1.5 0 0 1-1.5-1.5V5a3 3 0 0 0-3-3z" />,
    audio: <path d="M3 8v4h3l4 4V4L6 8H3zm10-2a4 4 0 0 1 0 8m2-10a7 7 0 0 1 0 12" />,
    effects: <><path d="m10 2 2 4.5L16.5 8l-4.5 1.5L10 14l-2-4.5L3.5 8l4.5-1.5z" /><path d="m4 14 1 2 2 1-2 1-1 2-1-2-2-1 2-1z" /></>,
    deliver: <><path d="M10 3v10m-4-4 4 4 4-4M3 15v3h14v-3" /></>,
    edit: <><circle cx="6" cy="6" r="2" /><circle cx="6" cy="14" r="2" /><path d="m8 7 9 9M8 13l9-9" /></>,
    keyframe: <><path d="M10 3l6 7-6 7-6-7z" fill="none" stroke="currentColor" strokeWidth="1.5" /><circle cx="10" cy="10" r="2" fill="currentColor" /></>,
    subtitle: <><rect x="3" y="4" width="14" height="11" rx="1.5" fill="none" stroke="currentColor" strokeWidth="1.5" /><path d="M6 8h8M6 11h5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" /><path d="M7 15l-2 3v-3" fill="none" stroke="currentColor" strokeWidth="1.5" /></>,
  };
  return <svg viewBox="0 0 20 20" aria-hidden="true">{paths[name]}</svg>;
}
