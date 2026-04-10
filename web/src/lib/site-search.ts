import { searchDocs, type DocLink } from './docs';

export type SearchResult = DocLink & {
    section: string;
};

export const STATIC_SEARCH_RESULTS: SearchResult[] = [
    { title: 'Home', path: '/', section: 'Pages' },
    { title: 'Documentation', path: '/docs/overview', section: 'Pages' },
    { title: 'Installation', path: '/install', section: 'Pages' },
    { title: 'Quick Start', path: '/docs/getting_started/quick_start', section: 'Pages' },
    { title: 'Changelog', path: '/changelog', section: 'Pages' },
    { title: 'Terms of Use', path: '/terms', section: 'Legal' },
    { title: 'Privacy Policy', path: '/privacy', section: 'Legal' },
];

export function searchSite(query: string): SearchResult[] {
    const normalizedQuery = query.trim().toLowerCase();
    const docsResults = searchDocs(query).map((item) => ({
        ...item,
        section: 'Documentation',
    }));

    if (!normalizedQuery) {
        return [...STATIC_SEARCH_RESULTS, ...docsResults].slice(0, 8);
    }

    const pageResults = STATIC_SEARCH_RESULTS.filter((item) => {
        const haystack = `${item.title} ${item.path} ${item.section}`.toLowerCase();
        return haystack.includes(normalizedQuery);
    });

    return [...pageResults, ...docsResults].slice(0, 8);
}
