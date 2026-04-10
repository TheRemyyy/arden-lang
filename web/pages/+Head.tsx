import { usePageContext } from 'vike-react/usePageContext';
import { getDocBreadcrumbs } from '../src/lib/docs';
import type { ChangelogPageData, DocsPageData } from '../src/lib/content.server';
import { ROUTE_OG_IMAGES } from '../src/lib/generated-og-images';
import {
    CURRENT_VERSION,
    FAVICON_SRC,
    GITHUB_REPO_URL,
    LICENSE_URL,
    OG_IMAGE_SRC,
    OG_LOGO_SRC,
    RSS_FEED_SRC,
    SITE_CREATOR_NAME,
    SITE_DESCRIPTION,
    SITE_LOCALE,
    SITE_NAME,
    SITE_ORGANIZATION_NAME,
    SITE_SEARCH_PATH,
    SITE_TWITTER_HANDLE,
    SITE_URL,
    UI_LOGO_SRC,
} from '../src/lib/site';

function toAbsoluteUrl(path: string): string {
    return path.startsWith('http') ? path : `${SITE_URL}${path}`;
}

function normalizeCanonicalUrl(urlPathname: string): string {
    return urlPathname === '/' ? SITE_URL : `${SITE_URL}${urlPathname}`;
}

function getPageTitle(pageContext: ReturnType<typeof usePageContext>): string {
    const data = pageContext.data as { title?: string } | undefined;
    if (typeof data?.title === 'string' && data.title.length > 0) {
        return data.title;
    }

    const title = pageContext.config.title;
    return typeof title === 'string' ? title : SITE_NAME;
}

function getPageDescription(pageContext: ReturnType<typeof usePageContext>): string {
    const data = pageContext.data as { description?: string } | undefined;
    if (typeof data?.description === 'string' && data.description.length > 0) {
        return data.description;
    }

    const description = pageContext.config.description;
    return typeof description === 'string' ? description : SITE_DESCRIPTION;
}

function getStructuredData(pageContext: ReturnType<typeof usePageContext>, canonicalUrl: string) {
    const docsData = pageContext.data as DocsPageData | undefined;
    const changelogData = pageContext.data as ChangelogPageData | undefined;
    const isDocsPage = pageContext.urlPathname.startsWith('/docs');
    const isChangelogPage = pageContext.urlPathname === '/changelog';
    const isHomePage = pageContext.urlPathname === '/';
    const isInstallPage = pageContext.urlPathname === '/install';
    const isPrivacyPage = pageContext.urlPathname === '/privacy';
    const isTermsPage = pageContext.urlPathname === '/terms';
    const breadcrumbs = isDocsPage ? getDocBreadcrumbs(pageContext.urlPathname) : null;

    const websiteId = `${SITE_URL}#website`;
    const organizationId = `${SITE_URL}#organization`;
    const softwareId = `${SITE_URL}#software`;

    const website = {
        '@context': 'https://schema.org',
        '@type': 'WebSite',
        '@id': websiteId,
        name: SITE_NAME,
        url: SITE_URL,
        description: SITE_DESCRIPTION,
        inLanguage: 'en',
        publisher: { '@id': organizationId },
        potentialAction: {
            '@type': 'SearchAction',
            target: `${SITE_URL}${SITE_SEARCH_PATH}?q={search_term_string}`,
            'query-input': 'required name=search_term_string',
        },
    };

    const organization = {
        '@context': 'https://schema.org',
        '@type': 'Organization',
        '@id': organizationId,
        name: SITE_ORGANIZATION_NAME,
        url: SITE_URL,
        logo: toAbsoluteUrl(OG_LOGO_SRC),
        sameAs: [GITHUB_REPO_URL],
    };

    const software = {
        '@context': 'https://schema.org',
        '@type': 'SoftwareSourceCode',
        '@id': softwareId,
        name: SITE_NAME,
        codeRepository: GITHUB_REPO_URL,
        programmingLanguage: SITE_NAME,
        url: SITE_URL,
        description: SITE_DESCRIPTION,
        runtimePlatform: 'LLVM',
        version: CURRENT_VERSION,
        license: LICENSE_URL,
        author: {
            '@type': 'Person',
            name: SITE_CREATOR_NAME,
        },
        publisher: { '@id': organizationId },
    };

    const pageSchemas = [];

    if (isHomePage) {
        pageSchemas.push({
            '@context': 'https://schema.org',
            '@type': 'WebPage',
            name: getPageTitle(pageContext),
            description: getPageDescription(pageContext),
            url: canonicalUrl,
            isPartOf: { '@id': websiteId },
            about: { '@id': softwareId },
        });
    }

    if (isDocsPage && docsData) {
        pageSchemas.push({
            '@context': 'https://schema.org',
            '@type': 'TechArticle',
            headline: docsData.title,
            description: docsData.description,
            url: canonicalUrl,
            dateModified: docsData.lastUpdated,
            author: {
                '@type': 'Person',
                name: SITE_CREATOR_NAME,
            },
            publisher: { '@id': organizationId },
            about: { '@id': softwareId },
            isPartOf: { '@id': websiteId },
        });
    }

    if (isChangelogPage && changelogData) {
        pageSchemas.push({
            '@context': 'https://schema.org',
            '@type': 'CollectionPage',
            name: changelogData.title,
            description: changelogData.description,
            url: canonicalUrl,
            dateModified: changelogData.lastUpdated,
            isPartOf: { '@id': websiteId },
            mainEntity: {
                '@type': 'ItemList',
                itemListElement: changelogData.releases.slice(0, 10).map((release, index) => ({
                    '@type': 'ListItem',
                    position: index + 1,
                    url: `${canonicalUrl}#${release.id}`,
                    name: release.label,
                    datePublished: release.date ?? undefined,
                })),
            },
        });
    }

    if (isInstallPage) {
        pageSchemas.push({
            '@context': 'https://schema.org',
            '@type': 'SoftwareApplication',
            name: `${SITE_NAME} Install`,
            operatingSystem: 'Linux, macOS, Windows',
            applicationCategory: 'DeveloperApplication',
            url: canonicalUrl,
            downloadUrl: 'https://github.com/TheRemyyy/arden-lang/releases/latest',
            offers: {
                '@type': 'Offer',
                price: '0',
                priceCurrency: 'USD',
            },
            isPartOf: { '@id': websiteId },
        });
    }

    if (isPrivacyPage || isTermsPage) {
        pageSchemas.push({
            '@context': 'https://schema.org',
            '@type': 'WebPage',
            name: getPageTitle(pageContext),
            description: getPageDescription(pageContext),
            url: canonicalUrl,
            isPartOf: { '@id': websiteId },
        });
    }

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

    return [website, organization, software, ...pageSchemas, breadcrumbSchema].filter(Boolean);
}

export default function Head() {
    const pageContext = usePageContext();
    const canonicalUrl = normalizeCanonicalUrl(pageContext.urlPathname);
    const title = getPageTitle(pageContext);
    const description = getPageDescription(pageContext);
    const routeOgImage = ROUTE_OG_IMAGES[pageContext.urlPathname as keyof typeof ROUTE_OG_IMAGES] ?? OG_IMAGE_SRC;
    const imageUrl = toAbsoluteUrl(routeOgImage);
    const isSearchPage = pageContext.urlPathname === SITE_SEARCH_PATH;
    const isErrorPage = Boolean(pageContext.is404 || pageContext.abortStatusCode || pageContext.errorWhileRendering);
    const robotsContent = isErrorPage || isSearchPage
        ? 'noindex, follow, max-image-preview:large, max-snippet:-1, max-video-preview:-1'
        : 'index, follow, max-image-preview:large, max-snippet:-1, max-video-preview:-1';

    return (
        <>
            <link rel="canonical" href={canonicalUrl} />
            <link rel="alternate" href={canonicalUrl} hrefLang="en" />
            <link rel="alternate" href={canonicalUrl} hrefLang="x-default" />
            <link rel="alternate" type="application/rss+xml" title={`${SITE_NAME} releases`} href={RSS_FEED_SRC} />
            <link rel="alternate" type="text/plain" title={`${SITE_NAME} llms.txt`} href="/llms.txt" />
            <link rel="alternate" type="text/plain" title={`${SITE_NAME} llms full index`} href="/llms-full.txt" />
            <link rel="icon" type="image/png" href={FAVICON_SRC} />
            <link rel="shortcut icon" href={FAVICON_SRC} />
            <link rel="preload" href={UI_LOGO_SRC} as="image" type="image/png" />
            <meta name="robots" content={robotsContent} />
            <meta name="googlebot" content={robotsContent} />
            <meta name="referrer" content="strict-origin-when-cross-origin" />
            <meta property="og:type" content="website" />
            <meta property="og:site_name" content={SITE_NAME} />
            <meta property="og:locale" content={SITE_LOCALE} />
            <meta property="og:url" content={canonicalUrl} />
            <meta property="og:image:secure_url" content={imageUrl} />
            <meta property="og:image:type" content="image/png" />
            <meta property="og:image:alt" content={`${SITE_NAME} social preview`} />
            <meta name="twitter:card" content="summary_large_image" />
            <meta name="twitter:title" content={title} />
            <meta name="twitter:description" content={description} />
            <meta name="twitter:url" content={canonicalUrl} />
            <meta name="twitter:image" content={imageUrl} />
            <meta name="twitter:image:alt" content={`${SITE_NAME} social preview`} />
            <meta name="twitter:site" content={SITE_TWITTER_HANDLE} />
            <meta name="twitter:creator" content={SITE_TWITTER_HANDLE} />
            <script
                type="application/ld+json"
                dangerouslySetInnerHTML={{
                    __html: JSON.stringify(getStructuredData(pageContext, canonicalUrl)),
                }}
            />
        </>
    );
}
