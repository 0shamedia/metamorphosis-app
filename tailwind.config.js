const defaultTheme = require('tailwindcss/defaultTheme'); // Import defaultTheme

/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    './src/**/*.{js,ts,jsx,tsx,mdx}',
  ],
  theme: {
    extend: {
      fontFamily: {
        // The 'sans' key can be left to default or just include fallbacks if
        // quicksand.className on body is the primary method of applying the font.
        // For robustness, ensuring Quicksand is first if 'font-sans' is used anywhere explicitly:
        sans: ['Quicksand', ...defaultTheme.fontFamily.sans],
        // If quicksand.className is on <body>, this might not even be strictly necessary
        // unless you use `font-sans` utility class explicitly on some elements
        // and want to ensure Quicksand is the primary font for that utility.
        // Or, if you want to define a custom font name:
        // quicksand: ['Quicksand', ...defaultTheme.fontFamily.sans],
      },
      colors: {
        purple: {
          50: '#f5f3ff',
          100: '#ede9fe',
          200: '#ddd6fe',
          300: '#c4b5fd',
          400: '#a78bfa',
          500: '#8b5cf6',
          600: '#7c3aed',
          700: '#6d28d9',
          800: '#5b21b6',
          900: '#4c1d95',
        },
        pink: {
          50: '#fdf2f8',
          100: '#fce7f3',
          200: '#fbcfe8',
          300: '#f9a8d4',
          400: '#f472b6',
          500: '#ec4899',
          600: '#db2777',
          700: '#be185d',
          800: '#9d174d',
          900: '#831843',
        },
      },
      width: {
        '120': '30rem',
      },
      animation: {
        'pulse-custom': 'pulse 2s ease-in-out infinite',
      },
    },
  },
  plugins: [],
}