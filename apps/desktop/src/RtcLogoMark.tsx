type RtcLogoMarkProps = {
  className?: string;
};

export function RtcLogoMark({ className }: RtcLogoMarkProps) {
  return (
    <svg
      aria-hidden="true"
      className={className}
      viewBox="0 0 24 24"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <path d="M7.5 7.75a4.25 4.25 0 0 0 0 8.5" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
      <circle cx="7.5" cy="12" r="1.55" fill="currentColor" />
      <path d="M16.5 7.75a4.25 4.25 0 0 1 0 8.5" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
      <circle cx="16.5" cy="12" r="1.55" fill="currentColor" />
      <path d="M9.05 12h1.45l.9-1.8 1.3 3.6 1.05-2.6.75.8h.95" stroke="currentColor" strokeWidth="1.35" strokeLinecap="round" strokeLinejoin="round" />
      <circle className="rtc-logo-status" cx="16.75" cy="5.65" r="1.25" />
    </svg>
  );
}
