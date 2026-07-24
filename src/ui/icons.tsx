import type { SVGProps } from "react";

type IconProps = SVGProps<SVGSVGElement>;

const commonProps = {
  "aria-hidden": true,
  fill: "none",
  focusable: false,
  viewBox: "0 0 24 24",
} as const;

export function FlagIcon(props: IconProps) {
  return (
    <svg {...commonProps} {...props}>
      <path
        d="M5 21V4.5C8.4 2.5 10.7 6.3 14 4.5c1.6-.9 3-1.5 5-1.5v11c-3.4 2-5.7-1.8-9-.1-1.5.8-3 1.5-5 1.6"
        stroke="currentColor"
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth="1.8"
      />
      <path
        d="M6.2 4.2c2.3-.3 4 1.5 6.1 1.2v3.8c-2.2.4-4-1.5-6.1-1.1V4.2Zm6.1 1.2c2-.2 3.4-1.7 5.5-1.7v3.8c-2 .1-3.5 1.6-5.5 1.7V5.4ZM6.2 8.1c2.1-.4 3.9 1.5 6.1 1.1V13c-2.2.4-4-1.5-6.1-1.1V8.1Zm6.1 1.1c2-.1 3.5-1.6 5.5-1.7v3.8c-1.9.1-3.5 1.6-5.5 1.7V9.2Z"
        fill="currentColor"
      />
    </svg>
  );
}

export function DashboardIcon(props: IconProps) {
  return (
    <svg {...commonProps} {...props}>
      <path
        d="M4 5.5h6v6H4v-6Zm10 0h6v3h-6v-3ZM4 15.5h6v3H4v-3Zm10-3h6v6h-6v-6Z"
        stroke="currentColor"
        strokeLinejoin="round"
        strokeWidth="1.7"
      />
    </svg>
  );
}

export function SettingsIcon(props: IconProps) {
  return (
    <svg {...commonProps} {...props}>
      <path
        d="M9.6 3.4 10 2h4l.4 1.4a8.8 8.8 0 0 1 1.4.8l1.4-.4 2 3.4-1 1a8.7 8.7 0 0 1 0 1.6l1 1-2 3.4-1.4-.4a8.8 8.8 0 0 1-1.4.8L14 16h-4l-.4-1.4a8.8 8.8 0 0 1-1.4-.8l-1.4.4-2-3.4 1-1a8.7 8.7 0 0 1 0-1.6l-1-1 2-3.4 1.4.4a8.8 8.8 0 0 1 1.4-.8Z"
        stroke="currentColor"
        strokeLinejoin="round"
        strokeWidth="1.5"
        transform="translate(0 3)"
      />
      <circle cx="12" cy="12" r="2.4" stroke="currentColor" strokeWidth="1.5" />
    </svg>
  );
}

export function PulseIcon(props: IconProps) {
  return (
    <svg {...commonProps} {...props}>
      <path
        d="M3 12h4l2-5 4 10 2-5h6"
        stroke="currentColor"
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth="1.8"
      />
    </svg>
  );
}

export function PlusIcon(props: IconProps) {
  return (
    <svg {...commonProps} {...props}>
      <path
        d="M12 5v14M5 12h14"
        stroke="currentColor"
        strokeLinecap="round"
        strokeWidth="1.8"
      />
    </svg>
  );
}

export function CheckIcon(props: IconProps) {
  return (
    <svg {...commonProps} {...props}>
      <circle cx="12" cy="12" r="9" stroke="currentColor" strokeWidth="1.7" />
      <path
        d="m8 12 2.7 2.7L16.5 9"
        stroke="currentColor"
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth="1.8"
      />
    </svg>
  );
}
