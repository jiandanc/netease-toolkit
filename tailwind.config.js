/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{vue,js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        apple: {
          red: "#FA2D48",
          bg: "#F5F5F7",
          card: "#FFFFFF",
          text: "#1D1D1F",
          secondary: "#86868B",
          green: "#34C759",
          gray: "#C7C7CC",
          divider: "#E5E5EA",
          blue: "#007AFF",
          purple: "#6366F1",
        },
      },
      fontFamily: {
        sans: [
          "SF Pro Display",
          "PingFang SC",
          "Microsoft YaHei",
          "system-ui",
          "-apple-system",
          "sans-serif",
        ],
      },
      borderRadius: {
        xl: "10px",
      },
      boxShadow: {
        card: "0 1px 3px rgba(0,0,0,0.06), 0 1px 2px rgba(0,0,0,0.04)",
        popover: "0 8px 30px rgba(0,0,0,0.12)",
      },
    },
  },
  plugins: [],
};
