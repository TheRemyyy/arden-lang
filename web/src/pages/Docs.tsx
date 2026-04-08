import { useEffect, useState } from 'react';
import { Menu, X } from 'lucide-react';
import { useLocation, useNavigate } from 'react-router-dom';
import {
    FLATTENED_DOCS,
    NAV_ITEMS,
    getCurrentSectionTitle,
    getDocNeighbors,
    normalizeDocsPath,
} from '../lib/docs';
import { renderMarkdown, rewriteInternalDocLinks } from '../lib/markdown';

type PageHeading = {
    id: string;
    text: string;
    level: 2 | 3;
};

function extractHeadings(html: string): PageHeading[] {
    const tempDiv = document.createElement('div');
    tempDiv.innerHTML = html;

    return Array.from(tempDiv.querySelectorAll('h2[id], h3[id]')).map((heading) => ({
        id: heading.getAttribute('id') ?? '',
        text: heading.textContent ?? '',
        level: heading.tagName === 'H2' ? 2 : 3,
    }));
}

function TableOfContents({ headings }: { headings: PageHeading[] }) {
    if (headings.length === 0) {
        return null;
    }

    return (
        <aside className="custom-scrollbar sticky top-24 hidden max-h-[calc(100vh-7rem)] overflow-y-auto xl:block">
            <div className="rounded-[1.6rem] border border-[rgba(57,52,46,0.12)] bg-[rgba(251,247,241,0.88)] p-5">
                <p className="text-xs font-semibold uppercase tracking-[0.22em] text-[var(--text-muted)]">On this page</p>
                <ul className="mt-4 space-y-3">
                    {headings.map((heading) => (
                        <li key={heading.id}>
                            <a
                                href={`#${heading.id}`}
                                className={`block leading-6 transition-colors hover:text-[var(--accent)] ${
                                    heading.level === 3 ? 'pl-3 text-[13px] text-white/42' : 'text-sm text-[var(--text-muted)]'
                                }`}
                            >
                                {heading.text}
                            </a>
                        </li>
                    ))}
                </ul>
            </div>
        </aside>
    );
}

export function Docs() {
    const location = useLocation();
    const navigate = useNavigate();
    const [content, setContent] = useState('');
    const [headings, setHeadings] = useState<PageHeading[]>([]);
    const [loading, setLoading] = useState(true);
    const [isSidebarOpen, setIsSidebarOpen] = useState(false);

    const normalizedPath = normalizeDocsPath(location.pathname);
    const fetchPath = `${normalizedPath}.md`;
    const { prevDoc, nextDoc } = getDocNeighbors(normalizedPath);
    const currentDocTitle =
        FLATTENED_DOCS.find((item) => item.path === normalizedPath)?.title ?? 'Documentation';

    useEffect(() => {
        setIsSidebarOpen(false);
        setLoading(true);
        setContent('');
        setHeadings([]);
        const controller = new AbortController();

        fetch(fetchPath, { signal: controller.signal })
            .then((response) => {
                if (!response.ok) {
                    throw new Error('Not found');
                }

                return response.text();
            })
            .then(async (markdown) => {
                const html = await renderMarkdown(markdown);
                const rewrittenHtml = rewriteInternalDocLinks(html, normalizedPath);
                setContent(rewrittenHtml);
                setHeadings(extractHeadings(rewrittenHtml));
                setLoading(false);
                window.scrollTo(0, 0);
            })
            .catch((error: unknown) => {
                if (error instanceof DOMException && error.name === 'AbortError') {
                    return;
                }

                console.error(error);
                const fallbackHtml = '<h1>Document not found</h1><p>The requested page could not be found.</p>';
                setContent(fallbackHtml);
                setHeadings([]);
                setLoading(false);
            });

        return () => controller.abort();
    }, [fetchPath, normalizedPath]);

    const handleContentClick = (event: React.MouseEvent<HTMLElement>) => {
        const target = event.target as HTMLElement;
        const link = target.closest('a[data-router-link="true"]') as HTMLAnchorElement | null;
        if (!link) {
            return;
        }

        const href = link.getAttribute('href');
        if (!href) {
            return;
        }

        event.preventDefault();
        navigate(href);
    };

    return (
        <div className="min-h-screen bg-[var(--bg)] pt-16 text-[var(--text)]">
            <div className="border-b border-[rgba(57,52,46,0.12)] bg-[rgba(251,247,241,0.84)]">
                <div className="mx-auto flex max-w-7xl items-center justify-between gap-4 px-6 py-4">
                    <div>
                        <p className="text-xs uppercase tracking-[0.22em] text-[var(--text-muted)]">
                            {getCurrentSectionTitle(normalizedPath)}
                        </p>
                        <p className="mt-1 text-lg font-semibold tracking-[-0.02em]">{currentDocTitle}</p>
                    </div>
                    <button
                        onClick={() => setIsSidebarOpen((current) => !current)}
                        className="inline-flex h-11 items-center gap-2 rounded-full border border-[rgba(57,52,46,0.14)] bg-white/70 px-4 text-sm font-medium text-[var(--text)] lg:hidden"
                    >
                        {isSidebarOpen ? <X className="h-4 w-4" /> : <Menu className="h-4 w-4" />}
                        Browse docs
                    </button>
                </div>
            </div>

            <div className="mx-auto grid max-w-7xl gap-8 px-6 py-8 lg:grid-cols-[280px_minmax(0,1fr)] xl:grid-cols-[280px_minmax(0,1fr)_240px]">
                <nav
                    className={`custom-scrollbar fixed inset-y-0 left-0 top-16 z-40 w-[290px] overflow-y-auto border-r border-[rgba(57,52,46,0.12)] bg-[var(--surface)] p-6 transition-transform duration-300 lg:sticky lg:top-24 lg:z-auto lg:block lg:h-[calc(100vh-7rem)] lg:rounded-[1.75rem] lg:border lg:translate-x-0 ${
                        isSidebarOpen ? 'translate-x-0' : '-translate-x-full'
                    }`}
                >
                    <div className="space-y-8 pb-16">
                        {NAV_ITEMS.map((section) =>
                            'items' in section ? (
                                <section key={section.title}>
                                    <p className="mb-3 text-xs font-semibold uppercase tracking-[0.22em] text-[var(--text-muted)]">
                                        {section.title}
                                    </p>
                                    <ul className="space-y-1.5">
                                        {section.items.map((item) => {
                                            const isActive = normalizedPath === item.path;

                                            return (
                                                <li key={item.path}>
                                                    <button
                                                        onClick={() => navigate(item.path)}
                                                        className={`w-full rounded-2xl px-4 py-3 text-left text-sm transition-colors ${
                                                            isActive
                                                                ? 'bg-[var(--bg-strong)] text-white'
                                                                : 'text-[var(--text-muted)] hover:bg-[var(--surface-soft)] hover:text-[var(--text)]'
                                                        }`}
                                                    >
                                                        {item.title}
                                                    </button>
                                                </li>
                                            );
                                        })}
                                    </ul>
                                </section>
                            ) : (
                                <section key={section.path}>
                                    <button
                                        onClick={() => navigate(section.path)}
                                        className={`w-full rounded-2xl px-4 py-3 text-left text-sm font-semibold transition-colors ${
                                            normalizedPath === section.path
                                                ? 'bg-[var(--bg-strong)] text-white'
                                                : 'bg-white/70 text-[var(--text)] hover:bg-[var(--surface-soft)]'
                                        }`}
                                    >
                                        {section.title}
                                    </button>
                                </section>
                            ),
                        )}
                    </div>
                </nav>

                {isSidebarOpen && (
                    <button
                        className="fixed inset-0 top-16 z-30 bg-[rgba(23,20,17,0.22)] lg:hidden"
                        onClick={() => setIsSidebarOpen(false)}
                        aria-label="Close documentation navigation"
                    />
                )}

                <main className="min-w-0">
                    {loading ? (
                        <div className="paper-panel rounded-[2rem] p-8">
                            <div className="animate-pulse space-y-6">
                                <div className="h-12 w-1/2 rounded-full bg-[var(--surface-soft)]" />
                                <div className="h-4 w-full rounded-full bg-[var(--surface-soft)]" />
                                <div className="h-4 w-5/6 rounded-full bg-[var(--surface-soft)]" />
                                <div className="h-4 w-4/6 rounded-full bg-[var(--surface-soft)]" />
                            </div>
                        </div>
                    ) : (
                        <>
                            <article
                                className="prose prose-invert prose-zinc max-w-none rounded-[2rem] border border-white/10 bg-[#161311] px-6 py-8 shadow-[0_24px_80px_rgba(0,0,0,0.28)] md:px-10 md:py-10
                                prose-headings:scroll-mt-24
                                prose-h1:font-display prose-h1:text-4xl prose-h1:font-bold prose-h1:tracking-[-0.04em] prose-h1:text-white
                                prose-h2:border-b prose-h2:border-white/10 prose-h2:pb-3 prose-h2:font-display prose-h2:text-3xl prose-h2:font-bold prose-h2:tracking-[-0.03em] prose-h2:text-white
                                prose-h3:font-display prose-h3:text-2xl prose-h3:font-semibold prose-h3:tracking-[-0.03em] prose-h3:text-[#f3ece3]
                                prose-p:text-[16px] prose-p:leading-8 prose-p:text-white/72
                                prose-strong:text-white
                                prose-a:text-[var(--accent-soft)] prose-a:no-underline hover:prose-a:text-white
                                prose-table:my-8 prose-table:w-full prose-table:border-collapse prose-table:text-left prose-thead:border-b prose-thead:border-white/12 prose-th:px-3 prose-th:pb-3 prose-th:text-xs prose-th:uppercase prose-th:tracking-[0.18em] prose-th:text-white/50 prose-td:border-b prose-td:border-white/8 prose-td:px-3 prose-td:py-3 prose-td:text-white/78
                                prose-code:border-0 prose-code:bg-transparent prose-code:px-0 prose-code:py-0 prose-code:text-[13px] prose-code:text-[#f2d6c8] prose-code:before:content-none prose-code:after:content-none
                                prose-pre:rounded-[1.5rem] prose-pre:border prose-pre:border-white/10 prose-pre:bg-[#211e1a] prose-pre:text-[#f7efe5]
                                prose-li:text-white/72
                                prose-blockquote:border-l-[var(--accent-soft)] prose-blockquote:text-white"
                                onClick={handleContentClick}
                                dangerouslySetInnerHTML={{ __html: content }}
                            />

                            <div className="mt-8 grid gap-4 sm:grid-cols-2">
                                <DocPagerCard direction="Previous" doc={prevDoc} align="left" onNavigate={navigate} />
                                <DocPagerCard direction="Next" doc={nextDoc} align="right" onNavigate={navigate} />
                            </div>
                        </>
                    )}
                </main>

                <TableOfContents headings={headings} />
            </div>
        </div>
    );
}

function DocPagerCard({
    direction,
    doc,
    align,
    onNavigate,
}: {
    direction: string;
    doc: { title: string; path: string } | null;
    align: 'left' | 'right';
    onNavigate: (path: string) => void;
}) {
    if (!doc) {
        return <div />;
    }

    return (
        <button
            onClick={() => onNavigate(doc.path)}
            className={`paper-panel rounded-[1.6rem] p-5 transition-transform hover:-translate-y-0.5 ${
                align === 'right' ? 'text-right' : 'text-left'
            }`}
        >
            <p className="text-xs font-semibold uppercase tracking-[0.22em] text-[var(--text-muted)]">{direction}</p>
            <p className="mt-3 text-lg font-semibold tracking-[-0.02em] text-[var(--text)]">{doc.title}</p>
        </button>
    );
}
