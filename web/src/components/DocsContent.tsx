import { useState } from 'react';
import { NAV_ITEMS, getCurrentSectionTitle } from '../lib/docs';
import type { DocsPageData, PageHeading } from '../lib/content.server';

function TableOfContents({ headings }: { headings: PageHeading[] }) {
    if (headings.length === 0) return null;

    return (
        <div className="custom-scrollbar fixed right-0 top-0 hidden h-full w-64 overflow-y-auto border-l border-white/10 bg-[#11100d] p-8 pt-24 xl:block">
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
    const currentSectionTitle = getCurrentSectionTitle(normalizedPath);

    return (
        <div className="flex min-h-screen flex-col bg-[#0f0d0b] pt-14 font-sans text-[#f3ece3] selection:bg-[rgba(184,92,56,0.28)] selection:text-white lg:flex-row lg:pt-16">
            <div className="fixed left-0 right-0 top-14 z-20 flex h-10 items-center border-b border-white/10 bg-[#11100d] px-4 lg:hidden">
                <button
                    onClick={() => setIsSidebarOpen((open) => !open)}
                    className="flex items-center gap-2 text-sm font-medium text-white"
                >
                    <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h16M4 18h16" />
                    </svg>
                    Menu
                </button>
                <span className="ml-auto font-mono text-xs text-white/45">{currentSectionTitle}</span>
            </div>

            <nav
                className={`fixed left-0 top-16 z-30 flex h-[calc(100vh-4rem)] w-72 flex-col border-r border-white/10 bg-[#11100d] transition-transform duration-300 ${
                    isSidebarOpen ? 'translate-x-0' : '-translate-x-full lg:translate-x-0'
                }`}
            >
                <div className="custom-scrollbar flex-1 space-y-8 overflow-y-auto p-6 pb-20">
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
                                                                ? 'border-white/10 bg-[#201d19] text-white'
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
                                            ? 'border-white/10 bg-[#201d19] text-white'
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
                    className="fixed inset-0 top-14 z-20 bg-black/45 backdrop-blur-sm lg:hidden"
                    onClick={() => setIsSidebarOpen(false)}
                />
            )}

            <div className="w-full flex-1 pt-10 lg:ml-72 lg:pt-0 xl:mr-64">
                <main className="mx-auto min-h-[80vh] w-full max-w-4xl px-6 py-10 md:px-12 lg:py-16">
                    <article
                        className="prose prose-invert prose-zinc max-w-none rounded-[2rem] border border-white/10 bg-[#161311] px-6 py-8 shadow-[0_24px_80px_rgba(0,0,0,0.28)] md:px-10 md:py-10
                            prose-headings:scroll-mt-24
                            prose-h1:mb-8 prose-h1:font-display prose-h1:text-4xl prose-h1:font-bold prose-h1:tracking-[-0.04em] prose-h1:text-white
                            prose-h2:mt-12 prose-h2:mb-6 prose-h2:border-b prose-h2:border-white/10 prose-h2:pb-3 prose-h2:font-display prose-h2:text-3xl prose-h2:font-bold prose-h2:tracking-[-0.03em] prose-h2:text-white
                            prose-h3:mt-8 prose-h3:mb-4 prose-h3:font-display prose-h3:text-2xl prose-h3:font-semibold prose-h3:tracking-[-0.03em] prose-h3:text-[#f3ece3]
                            prose-p:mb-6 prose-p:text-[16px] prose-p:leading-8 prose-p:text-white/72
                            prose-ul:my-6 prose-ul:list-disc prose-ul:pl-6 prose-li:mb-2 prose-li:text-white/72
                            prose-table:my-8 prose-table:w-full prose-table:border-collapse prose-table:text-left prose-thead:border-b prose-thead:border-white/12 prose-th:px-3 prose-th:pb-3 prose-th:text-xs prose-th:uppercase prose-th:tracking-[0.18em] prose-th:text-white/50 prose-td:border-b prose-td:border-white/8 prose-td:px-3 prose-td:py-3 prose-td:text-white/78
                            prose-strong:font-semibold prose-strong:text-white
                            prose-a:text-[var(--accent-soft)] prose-a:no-underline hover:prose-a:text-white
                            prose-code:border-0 prose-code:bg-transparent prose-code:px-0 prose-code:py-0 prose-code:text-[13px] prose-code:text-[#f2d6c8] prose-code:before:content-none prose-code:after:content-none
                            prose-pre:rounded-[1.5rem] prose-pre:border prose-pre:border-white/10 prose-pre:bg-[#211e1a] prose-pre:text-[#f7efe5] prose-pre:shadow-sm"
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
