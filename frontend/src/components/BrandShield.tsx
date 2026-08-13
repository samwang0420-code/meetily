'use client';

/**
 * BrandShield — 言镜 AI brand mark
 * 2026-08-06 redesign: 抛弃盾牌，改用 v2 同心环几何 (与 public/logo.png 1:1)
 * 3 同心环 (多层记忆) + 中心 "镜" 球 + 12 点红点 (in-meeting)
 * 配色: navy #1a4257/#091824 渐变 + teal #13A89E + 录音红 #ff5757
 */

export function BrandShield({ size = 40, className = '' }: { size?: number; className?: string }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 1024 1024"
      width={size}
      height={size}
      className={className}
      fill="none"
      aria-hidden="true"
    >
      <defs>
        <linearGradient id="bg" x1="50%" y1="0%" x2="50%" y2="100%">
          <stop offset="0%" stopColor="#1a4257"/>
          <stop offset="55%" stopColor="#0f2638"/>
          <stop offset="100%" stopColor="#091824"/>
        </linearGradient>
        <radialGradient id="glow" cx="30%" cy="25%" r="80%">
          <stop offset="0%" stopColor="#ffffff" stopOpacity="0.22"/>
          <stop offset="50%" stopColor="#ffffff" stopOpacity="0.06"/>
          <stop offset="100%" stopColor="#ffffff" stopOpacity="0"/>
        </radialGradient>
        <linearGradient id="ringOuter" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stopColor="#9bf0e8"/>
          <stop offset="50%" stopColor="#13A89E"/>
          <stop offset="100%" stopColor="#0d7d77"/>
        </linearGradient>
        <linearGradient id="ringMid" x1="20%" y1="0%" x2="80%" y2="100%">
          <stop offset="0%" stopColor="#ffffff" stopOpacity="0.95"/>
          <stop offset="100%" stopColor="#bff5ee" stopOpacity="0.7"/>
        </linearGradient>
        <linearGradient id="ringInner" x1="0%" y1="0%" x2="0%" y2="100%">
          <stop offset="0%" stopColor="#13A89E"/>
          <stop offset="100%" stopColor="#0d7d77"/>
        </linearGradient>
        <radialGradient id="mirror" cx="40%" cy="35%" r="65%">
          <stop offset="0%" stopColor="#bff5ee"/>
          <stop offset="35%" stopColor="#13A89E"/>
          <stop offset="100%" stopColor="#072030"/>
        </radialGradient>
        <radialGradient id="recHalo" cx="50%" cy="50%" r="50%">
          <stop offset="0%" stopColor="#ff5757" stopOpacity="0.65"/>
          <stop offset="60%" stopColor="#ff5757" stopOpacity="0.18"/>
          <stop offset="100%" stopColor="#ff5757" stopOpacity="0"/>
        </radialGradient>
      </defs>

      <rect x="0" y="0" width="1024" height="1024" rx="229" ry="229" fill="url(#bg)"/>
      <rect x="0" y="0" width="1024" height="1024" rx="229" ry="229" fill="url(#glow)"/>

      <g transform="translate(512 512)">
        <circle cx="0" cy="0" r="338" fill="none"
                stroke="url(#ringOuter)" strokeWidth="38"
                strokeLinecap="round"
                strokeDasharray="1700 247"
                transform="rotate(-90)"/>
        <circle cx="0" cy="0" r="246" fill="none"
                stroke="url(#ringMid)" strokeWidth="22"
                strokeLinecap="round"
                strokeDasharray="1159 386"
                transform="rotate(60)"/>
        <circle cx="0" cy="0" r="172" fill="#0a1c2a" fillOpacity="0.55"/>
        <circle cx="0" cy="0" r="172" fill="none"
                stroke="url(#ringInner)" strokeWidth="10"
                opacity="0.85"/>
        <circle cx="0" cy="0" r="100" fill="url(#mirror)"/>
        <circle cx="0" cy="0" r="78" fill="none"
                stroke="#ffffff" strokeWidth="3" strokeOpacity="0.35"/>
        <ellipse cx="-22" cy="-30" rx="18" ry="14" fill="#ffffff" fillOpacity="0.4"/>
        <ellipse cx="28" cy="32" rx="28" ry="14" fill="#000000" fillOpacity="0.18"/>

        <circle cx="0" cy="-338" r="58" fill="url(#recHalo)"/>
        <circle cx="0" cy="-338" r="24" fill="#ff5757"/>
        <circle cx="-7" cy="-345" r="8" fill="#ffffff" fillOpacity="0.85"/>
      </g>
    </svg>
  );
}
