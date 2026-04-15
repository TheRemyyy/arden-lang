import typography from '@tailwindcss/typography';

/** @type {import('tailwindcss').Config} */
export default {
    theme: {
        extend: {
            fontFamily: {
                sans: ['Space Grotesk', 'system-ui', 'sans-serif'],
                display: ['Space Grotesk', 'system-ui', 'sans-serif'],
                serif: ['Instrument Serif', 'Georgia', 'serif'],
            },
        },
    },
    plugins: [typography],
}
