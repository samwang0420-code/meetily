'use client';

export function BrandShield({ size = 40, className = '' }: { size?: number; className?: string }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 100 100"
      width={size}
      height={size}
      className={className}
      fill="none"
      aria-hidden="true"
    >
      <path d="M50 5 L90 18 L90 50 C90 75 70 90 50 96 C30 90 10 75 10 50 L10 18 Z" fill="#0B2545"/>
      <path d="M50 9 L86 21 L86 50 C86 73 68 87 50 92 C32 87 14 73 14 50 L14 21 Z" fill="#FFFFFF"/>
      <path d="M50 9 L86 21 L86 50 C86 73 68 87 50 92 L50 9 Z" fill="#13A89E"/>
      <path d="M50 9 L14 21 L14 50 C14 73 32 87 50 92 L50 9 Z" fill="#FFFFFF"/>
      <rect x="2" y="49" width="14" height="3" fill="#0B2545"/>
      <rect x="84" y="49" width="14" height="3" fill="#0B2545"/>
      <rect x="29" y="48" width="4" height="4" rx="2" fill="#0B2545"/>
      <rect x="37" y="42" width="4" height="16" rx="2" fill="#13A89E"/>
      <rect x="45" y="35" width="4" height="30" rx="2" fill="#0B2545"/>
      <rect x="53" y="42" width="4" height="16" rx="2" fill="#FFFFFF"/>
      <rect x="61" y="45" width="4" height="10" rx="2" fill="#FFFFFF"/>
      <circle cx="69" cy="50" r="2.5" fill="#FFFFFF"/>
    </svg>
  );
}
