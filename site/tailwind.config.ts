import type { Config } from "tailwindcss";

const config: Config = {
  content: [
    "./pages/**/*.{js,ts,jsx,tsx,mdx}",
    "./components/**/*.{js,ts,jsx,tsx,mdx}",
    "./app/**/*.{js,ts,jsx,tsx,mdx}",
  ],
  theme: {
    extend: {
      colors: {
        backgroundBlack: "#17171D",
        hackClubRed: "#EC3750",
        hackClubBlueShade: "#32323D",
        hackClubBlue: "#338EDA",
        burrowStroke: "#595959",
        burrowHover: "#3D3D3D",
      },
      fontFamily: {
        SpaceMono: ["var(--font-space-mono)"],
        Poppins: ["var(--font-poppins)"],
        PhantomSans: ["var(--font-phantom-sans)"],
      },
    },
  },
  plugins: [require("@headlessui/tailwindcss")({ prefix: "ui" })],
};
export default config;
