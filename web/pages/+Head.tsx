import { usePageContext } from 'vike-react/usePageContext';
import { getDocBreadcrumbs } from '../src/lib/docs';
import {
    CURRENT_VERSION,
    FAVICON_SRC,
    GITHUB_REPO_URL,
    OG_LOGO_SRC,
    RSS_FEED_SRC,
    SITE_CREATOR_NAME,
    SITE_DESCRIPTION,
    SITE_LOCALE,
    SITE_NAME,
    SITE_ORGANIZATION_NAME,
    SITE_TWITTER_HANDLE,
    SITE_URL,
    UI_LOGO_SRC,
} from '../src/lib/site';

function normalizeCanonicalUrl(urlPathname: string): string {
    if (urlPathname === '/') {
        return SITE_URL;
    }
    return `${SITE_URL}${urlPathname}`;
}

function getPageTitle(pageContext: ReturnType<typeof usePageContext>): string {
    const data = pageContext.data as { title?: string } | undefined;
    if (typeof data?.title === 'string' && data.title.length > 0) {
        return data.title;
    }

    const config = pageContext.config;
    const title = config.title;
    return typeof title === 'string' ? title : SITE_NAME;
}

function getPageDescription(pageContext: ReturnType<typeof usePageContext>): string {
    const data = pageContext.data as { description?: string } | undefined;
    if (typeof data?.description === 'string' && data.description.length > 0) {
        return data.description;
    }

    const config = pageContext.config;
    const description = config.description;
    return typeof description === 'string' ? description : SITE_DESCRIPTION;
}

function getStructuredData(pageContext: ReturnType<typeof usePageContext>, canonicalUrl: string) {
    const isDocsPage = pageContext.urlPathname.startsWith('/docs');
    const isChangelogPage = pageContext.urlPathname === '/changelog';
    const isHomePage = pageContext.urlPathname === '/';
    const breadcrumbs = isDocsPage ? getDocBreadcrumbs(pageContext.urlPathname) : null;

    const website = {
        '@context': 'https://schema.org',
        '@type': 'WebSite',
        name: SITE_NAME,
        url: SITE_URL,
        description: SITE_DESCRIPTION,
        inLanguage: 'en',
    };

    const organization = {
        '@context': 'https://schema.org',
        '@type': 'Organization',
        name: SITE_ORGANIZATION_NAME,
        url: SITE_URL,
        logo: `${SITE_URL}${OG_LOGO_SRC}`,
        sameAs: [GITHUB_REPO_URL],
    };

    const software = {
        '@context': 'https://schema.org',
        '@type': 'SoftwareSourceCode',
        name: SITE_NAME,
        codeRepository: GITHUB_REPO_URL,
        programmingLanguage: SITE_NAME,
        url: SITE_URL,
        description: SITE_DESCRIPTION,
        runtimePlatform: 'LLVM',
        version: CURRENT_VERSION,
        author: {
            '@type': 'Person',
            name: SITE_CREATOR_NAME,
        },
        publisher: {
            '@type': 'Organization',
            name: SITE_ORGANIZATION_NAME,
        },
    };

    const homePage = isHomePage
        ? {
              '@context': 'https://schema.org',
              '@type': 'WebPage',
              name: getPageTitle(pageContext),
              description: getPageDescription(pageContext),
              url: canonicalUrl,
              isPartOf: {
                  '@type': 'WebSite',
                  name: SITE_NAME,
                  url: SITE_URL,
              },
          }
        : null;

    const docsPage = isDocsPage
        ? {
              '@context': 'https://schema.org',
              '@type': 'TechArticle',
              headline: getPageTitle(pageContext),
              description: getPageDescription(pageContext),
              url: canonicalUrl,
              author: {
                  '@type': 'Person',
                  name: SITE_CREATOR_NAME,
              },
              publisher: {
                  '@type': 'Organization',
                  name: SITE_ORGANIZATION_NAME,
              },
              about: {
                  '@type': 'SoftwareSourceCode',
                  name: SITE_NAME,
                  url: SITE_URL,
              },
          }
        : null;

    const changelogPage = isChangelogPage
        ? {
              '@context': 'https://schema.org',
              '@type': 'CollectionPage',
              name: getPageTitle(pageContext),
              description: getPageDescription(pageContext),
              url: canonicalUrl,
              isPartOf: {
                  '@type': 'WebSite',
                  name: SITE_NAME,
                  url: SITE_URL,
              },
          }
        : null;

    const breadcrumbSchema = breadcrumbs
        ? {
              '@context': 'https://schema.org',
              '@type': 'BreadcrumbList',
              itemListElement: breadcrumbs.map((item, index) => ({
                  '@type': 'ListItem',
                  position: index + 1,
                  name: item.title,
                  item: `${SITE_URL}${item.path === '/' ? '' : item.path}`,
              })),
          }
        : null;

    return [website, organization, software, homePage, docsPage, changelogPage, breadcrumbSchema].filter(Boolean);
}

export default function Head() {
    const pageContext = usePageContext();
    const canonicalUrl = normalizeCanonicalUrl(pageContext.urlPathname);
    const title = getPageTitle(pageContext);
    const description = getPageDescription(pageContext);
    const structuredData = getStructuredData(pageContext, canonicalUrl);

    return (
        <>
            <link rel="canonical" href={canonicalUrl} />
            <link rel="alternate" type="application/rss+xml" title={`${SITE_NAME} releases`} href={RSS_FEED_SRC} />
            <link rel="icon" type="image/png" href={FAVICON_SRC} />
            <link rel="shortcut icon" href={FAVICON_SRC} />
            <link rel="preload" href={UI_LOGO_SRC} as="image" type="image/png" />
            <meta name="robots" content="index, follow, max-image-preview:large, max-snippet:-1, max-video-preview:-1" />
            <meta property="og:type" content="website" />
            <meta property="og:site_name" content={SITE_NAME} />
            <meta property="og:locale" content={SITE_LOCALE} />
            <meta property="og:url" content={canonicalUrl} />
            <meta property="og:image:alt" content={`${SITE_NAME} logo`} />
            <meta name="twitter:card" content="summary_large_image" />
            <meta name="twitter:title" content={title} />
            <meta name="twitter:description" content={description} />
            <meta name="twitter:url" content={canonicalUrl} />
            <meta name="twitter:image" content={`${SITE_URL}${OG_LOGO_SRC}`} />
            <meta name="twitter:site" content={SITE_TWITTER_HANDLE} />
            <meta name="twitter:creator" content={SITE_TWITTER_HANDLE} />
            <script
                type="application/ld+json"
                dangerouslySetInnerHTML={{
                    __html: JSON.stringify(structuredData),
                }}
            />
        </>
    );
}
