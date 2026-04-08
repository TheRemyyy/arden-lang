import generatedNavItems from './generated-docs.json';

export type DocLink = {
    title: string;
    path: string;
};

export type DocSection =
    | DocLink
    | {
          title: string;
          items: DocLink[];
      };

export const NAV_ITEMS = generatedNavItems as DocSection[];

export const FLATTENED_DOCS = NAV_ITEMS.flatMap((section) =>
    'items' in section ? section.items : [section],
);

export function normalizeDocsPath(pathname: string): string {
    if (pathname === '/docs' || pathname === '/docs/') {
        return '/docs/overview';
    }

    if (pathname.endsWith('/')) {
        return pathname.slice(0, -1);
    }

    return pathname;
}

export function getCurrentSectionTitle(pathname: string): string {
    const normalizedPath = normalizeDocsPath(pathname);
    const section = NAV_ITEMS.find((item) =>
        'items' in item
            ? item.items.some((entry) => entry.path === normalizedPath)
            : item.path === normalizedPath,
    );

    return section?.title ?? 'Documentation';
}

export function getDocNeighbors(pathname: string): {
    prevDoc: DocLink | null;
    nextDoc: DocLink | null;
} {
    const normalizedPath = normalizeDocsPath(pathname);
    const currentIndex = FLATTENED_DOCS.findIndex((item) => item.path === normalizedPath);

    return {
        prevDoc: currentIndex > 0 ? FLATTENED_DOCS[currentIndex - 1] : null,
        nextDoc:
            currentIndex !== -1 && currentIndex < FLATTENED_DOCS.length - 1
                ? FLATTENED_DOCS[currentIndex + 1]
                : null,
    };
}

export function getDocBreadcrumbs(pathname: string): DocLink[] {
    const normalizedPath = normalizeDocsPath(pathname);
    const breadcrumbs: DocLink[] = [
        { title: 'Home', path: '/' },
        { title: 'Documentation', path: '/docs/overview' },
    ];

    const section = NAV_ITEMS.find((item) =>
        'items' in item
            ? item.items.some((entry) => entry.path === normalizedPath)
            : item.path === normalizedPath,
    );

    if (!section) {
        return breadcrumbs;
    }

    if ('items' in section) {
        breadcrumbs.push({
            title: section.title,
            path: section.items[0]?.path ?? '/docs/overview',
        });

        const currentDoc = section.items.find((entry) => entry.path === normalizedPath);
        if (currentDoc) {
            breadcrumbs.push(currentDoc);
        }

        return breadcrumbs;
    }

    breadcrumbs.push(section);
    return breadcrumbs;
}

export function searchDocs(query: string): DocLink[] {
    const normalizedQuery = query.trim().toLowerCase();
    if (!normalizedQuery) {
        return [];
    }

    return FLATTENED_DOCS.filter((doc) => {
        const haystack = `${doc.title} ${doc.path}`.toLowerCase();
        return haystack.includes(normalizedQuery);
    }).slice(0, 8);
}
