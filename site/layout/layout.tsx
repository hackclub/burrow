import localFont from "next/font/local";

const phantomSans = localFont({
  src: [
    {
      path: "../assets/Regular.woff2",
      weight: "400",
      style: "normal",
    },
    {
      path: "../assets/Italic.woff2",
      weight: "400",
      style: "italic",
    },
    {
      path: "../assets/Bold.woff2",
      weight: "700",
      style: "normal",
    },
  ],
  variable: "--font-phantom-sans",
});

const fallbackFontVariables = {
  "--font-space-mono":
    '"SFMono-Regular", "SF Mono", ui-monospace, Menlo, Monaco, "Cascadia Mono", "Segoe UI Mono", "Roboto Mono", monospace',
  "--font-poppins":
    'var(--font-phantom-sans), -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
} as React.CSSProperties;

export default function Layout({ children }: { children: React.ReactNode }) {
  return (
    <div
      className={phantomSans.variable}
      style={fallbackFontVariables}
    >
      {children}
    </div>
  );
}
