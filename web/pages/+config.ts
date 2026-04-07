import vikeReact from 'vike-react/config';
import {
    APPLE_TOUCH_ICON_SRC,
    FAVICON_SRC,
    OG_LOGO_SRC,
    SITE_DESCRIPTION,
    SITE_NAME,
    SITE_URL,
} from '../src/lib/site';

export default {
    extends: [vikeReact],
    prerender: true,
    title: SITE_NAME,
    description: SITE_DESCRIPTION,
    image: `${SITE_URL}${OG_LOGO_SRC}`,
    favicon: FAVICON_SRC,
    lang: 'en',
    viewport: 'responsive',
    headHtmlEnd: `
      <meta name="theme-color" content="#0a0a0a" />
      <link rel="apple-touch-icon" sizes="180x180" href="${APPLE_TOUCH_ICON_SRC}" />
    `,
};
