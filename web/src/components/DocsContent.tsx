import { useState } from 'react';
import { NAV_ITEMS, getCurrentSectionTitle } from '../lib/docs';
import type { DocsPageData, PageHeading } from '../lib/content.server';

function TableOfContents({ headings }: { headings: PageHeading[] }) {
    if (headings.length === 0) return null;

    return (
        <div className="fixed right-0 top-0 hidden h-full w-64 overflow-y-auto border-l border-[#1f1f23] p-8 pt-24 xl:block">
            <h5 className="mb-4 text-xs font-bold uppercase tracking-widest text-gray-500">On This Page</h5>
            <ul className="space-y-3">
                {headings.map((heading) => (
                    <li key={heading.id}>
                        <a
                            href={`#${heading.id}`}
                            className="block text-[13px] leading-snug text-gray-400 transition-colors hover:text-[#a5b4fc] focus:text-white"
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
        <div className="flex min-h-screen flex-col bg-[#09090b] pt-14 font-sans text-gray-100 selection:bg-gray-700 selection:text-white lg:flex-row lg:pt-16">
            <div className="fixed left-0 right-0 top-14 z-20 flex h-10 items-center border-b border-[#1f1f23] bg-[#0c0c0e] px-4 lg:hidden">
                <button
                    onClick={() => setIsSidebarOpen((open) => !open)}
                    className="flex items-center gap-2 text-sm font-medium text-white"
                >
                    <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h16M4 18h16" />
                    </svg>
                    Menu
                </button>
                <span className="ml-auto font-mono text-xs text-gray-500">{currentSectionTitle}</span>
            </div>

            <nav
                className={`fixed left-0 top-16 z-30 flex h-[calc(100vh-4rem)] w-72 flex-col border-r border-[#1f1f23] bg-[#0c0c0e] transition-transform duration-300 ${
                    isSidebarOpen ? 'translate-x-0' : '-translate-x-full lg:translate-x-0'
                }`}
            >
                <div className="custom-scrollbar flex-1 space-y-8 overflow-y-auto p-6 pb-20">
                    {NAV_ITEMS.map((section) => (
                        <section key={section.title}>
                            {'items' in section ? (
                                <>
                                    <h3 className="mb-3 pl-2 text-[11px] font-bold uppercase tracking-widest text-gray-500">
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
                                                                ? 'border-[#27272a] bg-[#18181b] text-white'
                                                                : 'border-transparent text-gray-400 hover:bg-[#18181b]/50 hover:text-gray-200'
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
                                            ? 'border-[#27272a] bg-[#18181b] text-white'
                                            : 'border-transparent text-gray-500 hover:bg-[#18181b]/50 hover:text-gray-300'
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
                    className="fixed inset-0 top-14 z-20 bg-black/50 backdrop-blur-sm lg:hidden"
                    onClick={() => setIsSidebarOpen(false)}
                />
            )}

            <div className="w-full flex-1 pt-10 lg:ml-72 lg:pt-0 xl:mr-64">
                <main className="mx-auto min-h-[80vh] w-full max-w-4xl px-6 py-10 md:px-12 lg:py-16">
                    <article
                        className="prose prose-invert prose-zinc max-w-none
                            prose-headings:scroll-mt-24
                            prose-h1:mb-8 prose-h1:text-3xl prose-h1:font-bold prose-h1:tracking-tight prose-h1:text-white md:prose-h1:text-4xl
                            prose-h2:mt-12 prose-h2:mb-6 prose-h2:border-b prose-h2:border-[#27272a] prose-h2:pb-2 prose-h2:text-2xl prose-h2:font-semibold prose-h2:text-gray-100
                            prose-h3:mt-8 prose-h3:mb-4 prose-h3:text-xl prose-h3:font-semibold prose-h3:text-gray-200
                            prose-p:mb-6 prose-p:text-[15px] prose-p:leading-7 prose-p:text-gray-300 md:prose-p:text-[16px]
                            prose-ul:my-6 prose-ul:list-disc prose-ul:pl-6 prose-li:mb-2 prose-li:text-gray-300
                            prose-strong:font-semibold prose-strong:text-white
                            prose-code:rounded-md prose-code:border prose-code:border-[#27272a]/50 prose-code:bg-[#18181b] prose-code:px-1.5 prose-code:py-0.5 prose-code:text-[13px]
                            prose-pre:rounded-lg prose-pre:border prose-pre:border-[#27272a] prose-pre:bg-[#0c0c0e] prose-pre:shadow-sm"
                        dangerouslySetInnerHTML={{ __html: content }}
                    />

                    <div className="mt-16 flex flex-col justify-between gap-4 border-t border-[#27272a] pt-8 sm:flex-row">
                        <div>
                            {prevDoc && (
                                <a
                                    href={prevDoc.path}
                                    className="group flex w-full flex-col items-start gap-1 rounded-lg border border-[#27272a] p-4 transition-all hover:border-white/20 hover:bg-[#18181b] sm:w-auto"
                                >
                                    <span className="text-left text-xs font-medium uppercase tracking-wider text-gray-500 group-hover:text-gray-400">
                                        Previous
                                    </span>
                                    <span className="text-left font-medium text-gray-200 group-hover:text-white">
                                        {prevDoc.title}
                                    </span>
                                </a>
                            )}
                        </div>
                        <div className="flex justify-end text-right">
                            {nextDoc && (
                                <a
                                    href={nextDoc.path}
                                    className="group flex w-full flex-col items-end gap-1 rounded-lg border border-[#27272a] p-4 transition-all hover:border-white/20 hover:bg-[#18181b] sm:w-auto"
                                >
                                    <span className="text-right text-xs font-medium uppercase tracking-wider text-gray-500 group-hover:text-gray-400">
                                        Next
                                    </span>
                                    <span className="text-right font-medium text-gray-200 group-hover:text-white">
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
