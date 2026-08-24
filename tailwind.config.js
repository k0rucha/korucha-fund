/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ['./templates/**/*.html'],
  theme: {
    extend: {
      colors: {
        'da-blue': {
          1200: 'rgb(var(--da-blue-1200) / <alpha-value>)',
          900: 'rgb(var(--da-blue-900) / <alpha-value>)',
          600: 'rgb(var(--da-blue-600) / <alpha-value>)',
          50: 'rgb(var(--da-blue-50) / <alpha-value>)',
        },
        'da-orange': {
          600: 'rgb(var(--da-orange-600) / <alpha-value>)',
          400: 'rgb(var(--da-orange-400) / <alpha-value>)',
          50: 'rgb(var(--da-orange-50) / <alpha-value>)',
        },
        'da-gray': {
          800: 'rgb(var(--da-gray-800) / <alpha-value>)',
          600: 'rgb(var(--da-gray-600) / <alpha-value>)',
          400: 'rgb(var(--da-gray-400) / <alpha-value>)',
          200: 'rgb(var(--da-gray-200) / <alpha-value>)',
          50: 'rgb(var(--da-gray-50) / <alpha-value>)',
        },
      },
      fontFamily: {
        sans: ['system-ui', 'Hiragino Kaku Gothic ProN', 'Yu Gothic', 'Meiryo', 'sans-serif'],
      },
    },
  },
};
