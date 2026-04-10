import { useState } from 'react';
import { NAV_ITEMS, getCurrentSectionTitle, searchDocs } from '../lib/docs';
import type { DocsPageData, PageHeading } from '../lib/content.server';

const OVERVIEW_HIGHLIGHTS = [
    {
        label: 'Language Surface',
        title: 'Generics, interfaces, enums, ownership, async, and project mode',
        description: 'Start from the capabilities Arden already ships instead of guessing from the repo layout.',
        links: [
            { href: '/docs/advanced/generics', label: 'Generics' },
            { href: '/docs/features/interfaces', label: 'Interfaces' },
            { href: '/docs/advanced/ownership', label: 'Ownership' },
            { href: '/docs/advanced/async', label: 'Async' },
        ],
    },
    {
        label: 'Workflow',
        title: 'Installation, quick start, projects, testing, and CLI reference',
        description: 'Use the docs as a product map: install it, run it, then move into project mode and tooling.',
        links: [
            { href: '/docs/getting_started/installation', label: 'Installation' },
            { href: '/docs/getting_started/quick_start', label: 'Quick Start' },
            { href: '/docs/features/projects', label: 'Projects' },
            { href: '/docs/compiler/cli', label: 'CLI' },
        ],
    },
];

function TableOfContents({ headings }: { headings: PageHeading[] }) {
    if (headings.length === 0) return null;

    return (
        <div className="custom-scrollbar fixed right-0 top-0 hidden h-full w-64 overflow-y-auto border-l border-white/10 bg-[#1f1d1a] p-8 pt-24 xl:block">
            <h5 className="mb-4 text-xs font-bold uppercase tracking-widest text-white/45">On This Page</h5>
            <ul className="space-y-3">
                {headings.map((heading) => (
                    <li key={heading.id}>
                        <a
                            href={`#${heading.id}`}
                            className={`block text-[13px] leading-snug transition-colors hover:text-[var(--accent-soft)] focus:text-white ${
                                heading.level === 3 ? 'pl-3 text-white/42' : 'text-white/60'
                            }`}
                        >
                            {heading.text}
                        </a>
                    </li>
                ))}
            </ul>
        </div>
    );
}

export function DocsContent({
    content,
    headings,
    normalizedPath,
    prevDoc,
    nextDoc,
}: DocsPageData) {
    const [isSidebarOpen, setIsSidebarOpen] = useState(false);
    const [searchQuery, setSearchQuery] = useState('');
    const currentSectionTitle = getCurrentSectionTitle(normalizedPath);
    const searchResults = searchDocs(searchQuery);
    const isOverviewPage = normalizedPath === '/docs/overview';

    return (
        <div className="flex min-h-screen min-w-0 flex-col overflow-x-hidden bg-[#0f0d0b] pt-16 font-sans text-[#f3ece3] selection:bg-[rgba(184,92,56,0.28)] selection:text-white lg:flex-row lg:pt-16">
            <div className="fixed left-0 right-0 top-16 z-40 border-b border-white/10 bg-[#1f1d1a] lg:hidden">
                <div className="relative flex h-16 items-center gap-3 px-4">
                    <div className="relative min-w-0 flex-1">
                        <input
                            value={searchQuery}
                            onChange={(event) => setSearchQuery(event.target.value)}
                            type="text"
                            placeholder={`Search docs in ${currentSectionTitle}`}
                            className="h-11 w-full rounded-full border border-white/10 bg-white/[0.04] px-4 pr-10 text-sm text-white outline-none placeholder:text-white/35"
                        />
                        <svg className="pointer-events-none absolute right-4 top-1/2 h-4 w-4 -translate-y-1/2 text-white/35" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="m21 21-4.35-4.35M10.5 18a7.5 7.5 0 1 1 0-15 7.5 7.5 0 0 1 0 15Z" />
                        </svg>
                        {searchQuery.trim().length > 0 && (
                            <div className="absolute left-0 right-0 top-full z-30 mt-2 overflow-hidden rounded-[1.25rem] border border-white/10 bg-[#1f1d1a] shadow-[0_24px_80px_rgba(0,0,0,0.35)]">
                                {searchResults.length > 0 ? (
                                    <div className="custom-scrollbar max-h-80 overflow-y-auto py-2">
                                        {searchResults.map((result) => (
                                            <a
                                                key={result.path}
                                                href={result.path}
                                                onClick={() => setSearchQuery('')}
                                                className={`block px-4 py-3 text-sm transition-colors ${
                                                    result.path === normalizedPath
                                                        ? 'bg-white/[0.06] text-white'
                                                        : 'text-white/72 hover:bg-white/[0.04] hover:text-white'
                                                }`}
                                            >
                                                <span className="block font-medium">{result.title}</span>
                                                <span className="mt-1 block text-[11px] uppercase tracking-[0.16em] text-white/35">{result.path}</span>
                                            </a>
                                        ))}
                                    </div>
                                ) : (
                                    <div className="px-4 py-4 text-sm text-white/52">No matching docs found.</div>
                                )}
                            </div>
                        )}
                    </div>
                    <button
                        onClick={() => setIsSidebarOpen((open) => !open)}
                        className="inline-flex h-11 w-11 shrink-0 items-center justify-center rounded-full border border-white/10 bg-white/[0.04] text-white"
                        aria-label={isSidebarOpen ? 'Close documentation menu' : 'Open documentation menu'}
                    >
                        <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d={isSidebarOpen ? 'M6 18 18 6M6 6l12 12' : 'M4 6h16M4 12h16M4 18h16'} />
                        </svg>
                    </button>
                </div>
            </div>

            <nav
                className={`fixed bottom-0 left-0 top-32 z-30 flex w-[min(18rem,88vw)] flex-col overflow-hidden border-r border-white/10 bg-[#1f1d1a] transition-transform duration-300 lg:top-16 lg:h-[calc(100vh-4rem)] ${
                    isSidebarOpen ? 'translate-x-0' : '-translate-x-full lg:translate-x-0'
                }`}
            >
                <div className="custom-scrollbar flex-1 space-y-3 overflow-y-auto px-5 py-4">
                    {NAV_ITEMS.map((section) => (
                        <section key={section.title}>
                            {'items' in section ? (
                                <>
                                    <h3 className="mb-3 pl-2 text-[11px] font-bold uppercase tracking-widest text-white/45">
                                        {section.title}
                                    </h3>
                                    <ul className="space-y-0.5">
                                        {section.items.map((item) => {
                                            const isActive = normalizedPath === item.path;
                                            return (
                                                <li key={item.path}>
                                                    <a
                                                        href={item.path}
                                                        className={`block w-full rounded-md border px-3 py-1.5 text-left text-[14px] font-medium transition-colors duration-100 ${
                                                            isActive
                                                                ? 'border-white/10 bg-[#292621] text-white'
                                                                : 'border-transparent text-white/60 hover:bg-white/5 hover:text-white'
                                                        }`}
                                                        onClick={() => setIsSidebarOpen(false)}
                                                    >
                                                        {item.title}
                                                    </a>
                                                </li>
                                            );
                                        })}
                                    </ul>
                                </>
                            ) : (
                                <a
                                    href={section.path}
                                    className={`mb-2 block w-full rounded-md border px-3 py-1.5 text-left text-[14px] font-bold uppercase tracking-wider transition-colors duration-100 ${
                                        normalizedPath === section.path
                                            ? 'border-white/10 bg-[#292621] text-white'
                                            : 'border-transparent text-white/60 hover:bg-white/5 hover:text-white'
                                    }`}
                                    onClick={() => setIsSidebarOpen(false)}
                                >
                                    {section.title}
                                </a>
                            )}
                        </section>
                    ))}
                </div>
            </nav>

            {isSidebarOpen && (
                <div
                    className="fixed inset-0 top-16 z-20 bg-black/45 backdrop-blur-sm lg:hidden"
                    onClick={() => setIsSidebarOpen(false)}
                />
            )}

            <div className="w-full min-w-0 flex-1 pt-20 lg:ml-72 lg:pt-0 xl:mr-64">
                <main className="mx-auto min-h-[80vh] w-full max-w-4xl min-w-0 px-4 py-8 sm:px-6 md:px-12 lg:py-16">
                    {isOverviewPage && (
                        <section className="mb-12 grid gap-4 md:grid-cols-2">
                            {OVERVIEW_HIGHLIGHTS.map((group) => (
                                <article key={group.label} className="rounded-[1.75rem] border border-white/10 bg-white/[0.04] p-6">
                                    <p className="text-xs font-semibold uppercase tracking-[0.22em] text-white/40">
                                        {group.label}
                                    </p>
                                    <h2 className="mt-4 text-2xl font-semibold tracking-[-0.03em] text-white">
                                        {group.title}
                                    </h2>
                                    <p className="mt-3 text-sm leading-7 text-white/62">
                                        {group.description}
                                    </p>
                                    <div className="mt-5 flex flex-wrap gap-2">
                                        {group.links.map((link) => (
                                            <a
                                                key={link.href}
                                                href={link.href}
                                                className="inline-flex rounded-full border border-white/10 bg-white/[0.04] px-4 py-2 text-sm font-medium text-white/78 transition-colors hover:border-[var(--accent-soft)] hover:text-white"
                                            >
                                                {link.label}
                                            </a>
                                        ))}
                                    </div>
                                </article>
                            ))}
                        </section>
                    )}

                    <article
                        className="prose prose-invert prose-zinc max-w-none overflow-x-hidden px-0 py-0
                            prose-headings:scroll-mt-24
                            prose-h1:mb-8 prose-h1:font-display prose-h1:text-4xl prose-h1:font-bold prose-h1:tracking-[-0.04em] prose-h1:text-white
                            prose-h2:mt-12 prose-h2:mb-6 prose-h2:border-b prose-h2:border-white/10 prose-h2:pb-3 prose-h2:font-display prose-h2:text-3xl prose-h2:font-bold prose-h2:tracking-[-0.03em] prose-h2:text-white
                            prose-h3:mt-8 prose-h3:mb-4 prose-h3:font-display prose-h3:text-2xl prose-h3:font-semibold prose-h3:tracking-[-0.03em] prose-h3:text-[#f3ece3]
                            prose-p:mb-6 prose-p:text-[16px] prose-p:leading-8 prose-p:text-white/72
                            prose-ul:my-6 prose-ul:list-disc prose-ul:pl-6 prose-li:mb-2 prose-li:text-white/72
                            prose-img:max-w-full
                            prose-table:my-8 prose-table:block prose-table:max-w-full prose-table:overflow-x-auto prose-table:border-collapse prose-table:text-left prose-thead:border-b prose-thead:border-white/12 prose-th:px-3 prose-th:pb-3 prose-th:text-xs prose-th:uppercase prose-th:tracking-[0.18em] prose-th:text-white/50 prose-td:border-b prose-td:border-white/8 prose-td:px-3 prose-td:py-3 prose-td:text-white/78
                            prose-strong:font-semibold prose-strong:text-white
                            prose-a:text-[var(--accent-soft)] prose-a:no-underline hover:prose-a:text-white
                            prose-code:border-0 prose-code:bg-transparent prose-code:px-0 prose-code:py-0 prose-code:text-[13px] prose-code:text-[#f2d6c8] prose-code:before:content-none prose-code:after:content-none
                            prose-pre:max-w-full prose-pre:overflow-x-auto prose-pre:rounded-[1.5rem] prose-pre:border prose-pre:border-white/10 prose-pre:bg-[#292621] prose-pre:text-[#f7efe5] prose-pre:shadow-sm"
                        dangerouslySetInnerHTML={{ __html: content }}
                    />

                    <div className="mt-16 flex flex-col justify-between gap-4 border-t border-white/10 pt-8 sm:flex-row">
                        <div>
                            {prevDoc && (
                                <a
                                    href={prevDoc.path}
                                    className="group flex w-full flex-col items-start gap-1 rounded-2xl border border-white/10 bg-white/[0.04] p-4 transition-all hover:border-[var(--accent-soft)] hover:bg-white/[0.07] sm:w-auto"
                                >
                                    <span className="text-left text-xs font-medium uppercase tracking-wider text-white/45 group-hover:text-[var(--accent-soft)]">
                                        Previous
                                    </span>
                                    <span className="text-left font-medium text-white">
                                        {prevDoc.title}
                                    </span>
                                </a>
                            )}
                        </div>
                        <div className="flex justify-end text-right">
                            {nextDoc && (
                                <a
                                    href={nextDoc.path}
                                    className="group flex w-full flex-col items-end gap-1 rounded-2xl border border-white/10 bg-white/[0.04] p-4 transition-all hover:border-[var(--accent-soft)] hover:bg-white/[0.07] sm:w-auto"
                                >
                                    <span className="text-right text-xs font-medium uppercase tracking-wider text-white/45 group-hover:text-[var(--accent-soft)]">
                                        Next
                                    </span>
                                    <span className="text-right font-medium text-white">
                                        {nextDoc.title}
                                    </span>
                                </a>
                            )}
                        </div>
                    </div>

                    <TableOfContents headings={headings} />
                </main>
            </div>
        </div>
    );
}
