import { Space_Mono, Poppins } from "next/font/google";
import localFont from "next/font/local";

const space_mono = Space_Mono({
  weight: ["400", "700"],
  subsets: ["latin"],
  display: "swap",
  variable: "--font-space-mono",
});

const poppins = Poppins({
  weight: ["400", "500", "600", "700", "800", "900"],
  subsets: ["latin"],
  display: "swap",
  variable: "--font-poppins",
});

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

export default function Layout({ children }: { children: React.ReactNode }) {
  return (
    <div
      className={`${space_mono.variable} ${poppins.variable} ${phantomSans.variable}`}
    >
      {children}
    </div>
  );
}
