export default function Logo({ className = 'w-8 h-8' }: { className?: string }) {
  return (
    <svg viewBox="0 0 32 32" className={className} aria-hidden="true">
      <line x1="16" y1="16" x2="7" y2="9" stroke="#a5f3fc" strokeWidth="2.5" strokeLinecap="round" />
      <line x1="16" y1="16" x2="25" y2="9" stroke="#a5f3fc" strokeWidth="2.5" strokeLinecap="round" />
      <line x1="16" y1="16" x2="16" y2="26" stroke="#a5f3fc" strokeWidth="2.5" strokeLinecap="round" />
      <circle cx="16" cy="16" r="5" fill="#0891b2" />
      <circle cx="7" cy="9" r="3.5" fill="#0e7490" />
      <circle cx="25" cy="9" r="3.5" fill="#0e7490" />
      <circle cx="16" cy="26" r="3.5" fill="#7c3aed" />
    </svg>
  )
}
