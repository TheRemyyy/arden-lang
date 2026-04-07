import { usePageContext } from 'vike-react/usePageContext';
import { SITE_URL } from '../src/lib/site';

function normalizeCanonicalUrl(urlPathname: string): string {
    if (urlPathname === '/') {
        return SITE_URL;
    }
    return `${SITE_URL}${urlPathname}`;
}

export default function Head() {
    const pageContext = usePageContext();
    const canonicalUrl = normalizeCanonicalUrl(pageContext.urlPathname);

    return (
        <>
            <link rel="canonical" href={canonicalUrl} />
            <meta property="og:url" content={canonicalUrl} />
            <meta name="twitter:url" content={canonicalUrl} />
        </>
    );
}
