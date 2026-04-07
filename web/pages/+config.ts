import vikeReact from 'vike-react/config';
import {
    APPLE_TOUCH_ICON_SRC,
    FAVICON_SRC,
    OG_LOGO_SRC,
    SITE_DESCRIPTION,
    SITE_KEYWORDS,
    SITE_NAME,
    SITE_TITLE,
    SITE_URL,
    WEB_MANIFEST_SRC,
} from '../src/lib/site';

export default {
    extends: [vikeReact],
    prerender: true,
    title: SITE_TITLE,
    description: SITE_DESCRIPTION,
    image: `${SITE_URL}${OG_LOGO_SRC}`,
    favicon: FAVICON_SRC,
    lang: 'en',
    viewport: 'responsive',
    headHtmlEnd: `
      <meta name="theme-color" content="#0a0a0a" />
      <meta name="keywords" content="${SITE_KEYWORDS.join(', ')}" />
      <meta name="author" content="${SITE_NAME}" />
      <link rel="apple-touch-icon" sizes="180x180" href="${APPLE_TOUCH_ICON_SRC}" />
      <link rel="manifest" href="${WEB_MANIFEST_SRC}" />
    `,
};
