/** Vector disk artwork stays crisp at every desktop scale. */
export default function DiskArt({ small = false }: { small?: boolean }) {
  const id = small ? "disk-small" : "disk-hero";
  return (
    <svg className={small ? "disk-mini" : "disk-illustration"} viewBox="0 0 260 220" fill="none" aria-hidden="true">
      <defs>
        <linearGradient id={`${id}-body`} x1="70" y1="30" x2="200" y2="190" gradientUnits="userSpaceOnUse"><stop stopColor="#f1faff"/><stop offset=".5" stopColor="#a3d0ff"/><stop offset="1" stopColor="#4488fa"/></linearGradient>
        <linearGradient id={`${id}-face`} x1="100" y1="50" x2="190" y2="170" gradientUnits="userSpaceOnUse"><stop stopColor="#50baff"/><stop offset="1" stopColor="#0563ed"/></linearGradient>
        <filter id={`${id}-shadow`} x="-50%" y="-50%" width="200%" height="220%"><feDropShadow dx="0" dy="12" stdDeviation="10" floodColor="#398df4" floodOpacity=".24"/></filter>
      </defs>
      {!small && <g><ellipse cx="139" cy="174" rx="107" ry="34" transform="rotate(-20 139 174)" fill="#d9efff"/><ellipse cx="137" cy="158" rx="111" ry="42" transform="rotate(-25 137 158)" stroke="white" strokeWidth="2"/><path d="m218 31 3 9 9 3-9 3-3 9-3-9-9-3 9-3Z" fill="white"/><circle cx="50" cy="93" r="4" fill="#9dceff"/></g>}
      <g transform={small ? "translate(4 -4)" : "rotate(13 140 115)"} filter={`url(#${id}-shadow)`}>
        <path d="M83 35h109a14 14 0 0 1 14 14v132a14 14 0 0 1-14 14H83a14 14 0 0 1-14-14V49a14 14 0 0 1 14-14Z" fill="#5a9afa"/>
        <rect x="62" y="27" width="137" height="158" rx="16" fill={`url(#${id}-body)`} stroke="#eaf7ff" strokeWidth="2"/>
        <rect x="78" y="44" width="106" height="115" rx="12" fill={`url(#${id}-face)`} stroke="#d1eaff" strokeWidth="2"/>
        <path d="m109 77 20-3v23h-20Zm24-4 24-4v28h-24Zm-24 28h20v23l-20-3Zm24 0h24v28l-24-4Z" fill="#fff" fillOpacity=".95"/>
        <rect x="84" y="168" width="55" height="7" rx="3.5" fill="#569beb"/>
        <circle cx="176" cy="172" r="3" fill="#d0ffe9"/>
        <circle cx="72" cy="38" r="2" fill="#90b7e7"/><circle cx="189" cy="38" r="2" fill="#90b7e7"/>
      </g>
    </svg>
  );
}
