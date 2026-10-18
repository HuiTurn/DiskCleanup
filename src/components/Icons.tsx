import type { ReactNode } from "react";

export interface IconProps { size?: number; color?: string }
/** One optical grid: 24 units, rounded 1.7-unit strokes, 2-unit safe area. */
function Glyph({ size = 20, color, children }: IconProps & { children: ReactNode }) {
  return <svg className="ui-icon" width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.7} strokeLinecap="round" strokeLinejoin="round" style={{ color, flexShrink: 0, verticalAlign: "middle" }} aria-hidden="true" focusable="false">{children}</svg>;
}
const tint = { fill: "currentColor", fillOpacity: .12 };
export const IconHome = (p: IconProps) => <Glyph {...p}><path {...tint} d="m3.5 10 7-6a2.3 2.3 0 0 1 3 0l7 6v9a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2Z"/><path d="M9 21v-6a1.5 1.5 0 0 1 1.5-1.5h3A1.5 1.5 0 0 1 15 15v6M2.5 11l1-1m17 0 1 1"/></Glyph>;
export const IconScan = (p: IconProps) => <Glyph {...p}><circle {...tint} cx="10.5" cy="10.5" r="7"/><path d="m16 16 4.5 4.5M7 12v-2m3.5 2V7m3.5 5V9"/></Glyph>;
export const IconTrash = (p: IconProps) => <Glyph {...p}><path {...tint} d="m5.5 7 1 12a2 2 0 0 0 2 2h7a2 2 0 0 0 2-2l1-12"/><path d="M3.5 7h17M9 7V4.5A1.5 1.5 0 0 1 10.5 3h3A1.5 1.5 0 0 1 15 4.5V7M10 11v6m4-6v6"/></Glyph>;
export const IconFile = (p: IconProps) => <Glyph {...p}><path {...tint} d="M13 3H6.5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h11a2 2 0 0 0 2-2V9Z"/><path d="M13 3v4a2 2 0 0 0 2 2h4.5M8.5 13h7m-7 4h4"/></Glyph>;
export const IconCopy = (p: IconProps) => <Glyph {...p}><rect {...tint} x="8" y="8" width="13" height="13" rx="3"/><path d="M15.5 5V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v8.5a2 2 0 0 0 2 2M12 12.5h5m-5 4h3"/></Glyph>;
export const IconRocket = (p: IconProps) => <Glyph {...p}><path {...tint} d="M8 15c-1.5-6 5-12 12-12 0 7-6 13.5-12 12Z"/><circle cx="15.5" cy="7.5" r="1.5"/><path d="m8 9-4 1-1 5 5-1m6 1-1 5-5 1 1-5M6 17l-3 4m4-2-1 2"/></Glyph>;
export const IconTools = (p: IconProps) => <Glyph {...p}><path {...tint} d="M14 3.5a5 5 0 0 0-5.5 6.8l-5 5a3 3 0 0 0 4.2 4.2l5-5a5 5 0 0 0 6.8-5.5l-3.2 2-3.3-3.3Z"/><path d="m6.2 17.8.1-.1"/></Glyph>;
export const IconSettings = (p: IconProps) => <Glyph {...p}><path {...tint} d="m10 3-.7 2.3-1.5.9-2.3-.5-2 3.5L5 11v2l-1.5 1.8 2 3.5 2.3-.5 1.5.9L10 21h4l.7-2.3 1.5-.9 2.3.5 2-3.5L19 13v-2l1.5-1.8-2-3.5-2.3.5-1.5-.9L14 3Z"/><circle cx="12" cy="12" r="3.2"/></Glyph>;
export const IconBroom = (p: IconProps) => <Glyph {...p}><path {...tint} d="m8 10 8 4-3.5 6a1.8 1.8 0 0 1-2.4.7l-6-3a1.5 1.5 0 0 1-.5-2.3Z"/><path d="m11.5 11.8 5-8.8M8 15l-2 4m5-2-2 3M19 8v3m-1.5-1.5h3"/></Glyph>;
export const IconRefresh = (p: IconProps) => <Glyph {...p}><path d="M20 10a8 8 0 0 0-13.8-5L3.5 8M4 14a8 8 0 0 0 13.8 5l2.7-3M3.5 3.5V8H8m8 8h4.5v4.5"/></Glyph>;
export const IconCheck = (p: IconProps) => <Glyph {...p}><path d="m5 12 4.5 4.5L19 7"/></Glyph>;
export const IconClock = (p: IconProps) => <Glyph {...p}><circle {...tint} cx="12" cy="12" r="9"/><path d="M12 6.5V12l3.5 2"/></Glyph>;
export const IconChart = (p: IconProps) => <Glyph {...p}><rect {...tint} x="4" y="12" width="4" height="8" rx="1.5"/><rect {...tint} x="10" y="4" width="4" height="16" rx="1.5"/><rect {...tint} x="16" y="8" width="4" height="12" rx="1.5"/></Glyph>;
export const IconFolder = (p: IconProps) => <Glyph {...p}><path {...tint} d="M3 8V6a2 2 0 0 1 2-2h4l2 3h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z"/><path d="M3 9h18"/></Glyph>;
export const IconMove = (p: IconProps) => <Glyph {...p}><path {...tint} d="M10.5 20H5a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h4l2 3h8a2 2 0 0 1 2 2v2M3 9h18"/><path d="M12 17h9m-3.5-3.5L21 17l-3.5 3.5"/></Glyph>;
export const IconPower = (p: IconProps) => <Glyph {...p}><path {...tint} d="M7 5a8.5 8.5 0 1 0 10 0"/><path d="M12 3v9"/></Glyph>;
export const IconDrive = (p: IconProps) => <Glyph {...p}><path {...tint} d="M3 15 5.2 5.5A2 2 0 0 1 7.2 4h9.6a2 2 0 0 1 2 1.5L21 15v3a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z"/><path d="M3 14h18M7 17h6m4 0h.1"/></Glyph>;
export const IconShield = (p: IconProps) => <Glyph {...p}><path {...tint} d="m12 3 8 3v5c0 5-3.5 8.5-8 10-4.5-1.5-8-5-8-10V6Z"/><path d="m8.5 11.5 2.5 2.5 4.5-5"/></Glyph>;
export const IconInfo = (p: IconProps) => <Glyph {...p}><circle {...tint} cx="12" cy="12" r="9"/><path d="M12 11v6m0-10v.1"/></Glyph>;
export const IconImage = (p: IconProps) => <Glyph {...p}><rect {...tint} x="3" y="3" width="18" height="18" rx="3"/><circle cx="8" cy="8" r="1.5"/><path d="m3 17 5-5 4 4 3-3 6 5"/></Glyph>;
export const IconVideo = (p: IconProps) => <Glyph {...p}><rect {...tint} x="3" y="4" width="18" height="16" rx="3"/><path {...tint} d="m10 8 6 4-6 4Z"/></Glyph>;
export const IconAudio = (p: IconProps) => <Glyph {...p}><path {...tint} d="M9 7v11m0-11 11-3v11M9 10l11-3"/><ellipse {...tint} cx="6" cy="18" rx="3" ry="2.5"/><ellipse {...tint} cx="17" cy="15" rx="3" ry="2.5"/></Glyph>;
export const IconArchive = (p: IconProps) => <Glyph {...p}><rect {...tint} x="4" y="3" width="16" height="18" rx="3"/><path d="M11 3v2h2v3h-2v3h2v3m-2 0h2v3h-2Z"/></Glyph>;
export const IconApp = (p: IconProps) => <Glyph {...p}><rect {...tint} x="3" y="3" width="18" height="18" rx="5"/><path d="M3 9h18M7 6h.1m3 0h.1m-3 7h4v4H7Zm8 0h2m-2 4h2"/></Glyph>;
export const IconArrow = (p: IconProps) => <Glyph {...p}><path d="M4 12h16m-6-6 6 6-6 6"/></Glyph>;
