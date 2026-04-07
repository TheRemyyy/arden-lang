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

export const NAV_ITEMS: DocSection[] = [
    { title: 'Overview', path: '/docs/overview' },
    {
        title: 'Getting Started',
        items: [
            { title: 'Installation', path: '/docs/getting_started/installation' },
            { title: 'Quick Start', path: '/docs/getting_started/quick_start' },
            { title: 'Editor Setup', path: '/docs/getting_started/editor_setup' },
        ],
    },
    {
        title: 'Basics',
        items: [
            { title: 'Syntax', path: '/docs/basics/syntax' },
            { title: 'Variables', path: '/docs/basics/variables' },
            { title: 'Types', path: '/docs/basics/types' },
            { title: 'Control Flow', path: '/docs/basics/control_flow' },
        ],
    },
    {
        title: 'Features',
        items: [
            { title: 'Functions', path: '/docs/features/functions' },
            { title: 'Classes', path: '/docs/features/classes' },
            { title: 'Interfaces', path: '/docs/features/interfaces' },
            { title: 'Enums', path: '/docs/features/enums' },
            { title: 'Ranges', path: '/docs/features/ranges' },
            { title: 'Modules', path: '/docs/features/modules' },
            { title: 'Projects', path: '/docs/features/projects' },
        ],
    },
    {
        title: 'Standard Library',
        items: [
            { title: 'Overview', path: '/docs/stdlib/overview' },
            { title: 'Math', path: '/docs/stdlib/math' },
            { title: 'Str', path: '/docs/stdlib/string' },
            { title: 'Time', path: '/docs/stdlib/time' },
            { title: 'Args', path: '/docs/stdlib/args' },
            { title: 'Collections', path: '/docs/stdlib/collections' },
            { title: 'I/O', path: '/docs/stdlib/io' },
            { title: 'System', path: '/docs/stdlib/system' },
        ],
    },
    {
        title: 'Advanced',
        items: [
            { title: 'Ownership', path: '/docs/advanced/ownership' },
            { title: 'Generics', path: '/docs/advanced/generics' },
            { title: 'Async/Await', path: '/docs/advanced/async' },
            { title: 'Error Handling', path: '/docs/advanced/error_handling' },
            { title: 'Memory Management', path: '/docs/advanced/memory_management' },
        ],
    },
    {
        title: 'Compiler',
        items: [
            { title: 'CLI', path: '/docs/compiler/cli' },
            { title: 'Architecture', path: '/docs/compiler/architecture' },
        ],
    },
];

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
