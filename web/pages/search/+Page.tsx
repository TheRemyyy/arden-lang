import Search from 'lucide-react/dist/esm/icons/search';
import { useEffect, useMemo, useState } from 'react';
import { usePageContext } from 'vike-react/usePageContext';
import { searchSite } from '../../src/lib/site-search';
import type { SearchPageData } from './+data';

export default function Page() {
    const pageContext = usePageContext();
    const pageData = pageContext.data as SearchPageData | undefined;
    const serverQuery = pageData?.initialQuery ?? '';
    const serverResults = pageData?.initialResults ?? [];
    const [query, setQuery] = useState(serverQuery);

    useEffect(() => {
        setQuery(serverQuery);
    }, [serverQuery]);

    useEffect(() => {
        if (typeof window === 'undefined') {
            return;
        }

        const nextUrl = query ? `/search?q=${encodeURIComponent(query)}` : '/search';
        window.history.replaceState(null, '', nextUrl);
    }, [query]);

    const results = useMemo(() => {
        if (query === serverQuery) {
            return serverResults;
        }

        return searchSite(query);
    }, [query, serverQuery, serverResults]);

    return (
        <div className="mx-auto flex min-h-screen w-full max-w-4xl px-6 pb-16 pt-28 md:px-10 md:pb-20 md:pt-32">
            <section className="paper-panel w-full rounded-[2rem] p-6 md:p-8">
                <p className="text-xs font-semibold uppercase tracking-[0.24em] text-[var(--accent)]">
                    Site Search
                </p>
                <h1 className="mt-4 font-display text-4xl font-bold tracking-[-0.04em] text-[var(--text)] md:text-5xl">
                    Search Arden
                </h1>
                <p className="mt-4 max-w-2xl text-sm leading-7 text-[var(--text-muted)] md:text-base">
                    Search documentation, installation guides, changelog entries, and legal pages from one place.
                </p>

                <div className="mt-8 flex items-center gap-3 rounded-[1.5rem] border border-white/10 bg-[#11100d] px-4 py-4 shadow-[0_20px_60px_rgba(0,0,0,0.28)]">
                    <Search className="h-5 w-5 shrink-0 text-white/60" />
                    <input
                        autoFocus
                        value={query}
                        onChange={(event) => setQuery(event.target.value)}
                        type="text"
                        placeholder="Search docs, install, changelog..."
                        className="w-full appearance-none bg-transparent text-base text-white caret-white outline-none placeholder:text-white/35"
                    />
                </div>

                <div className="mt-6 overflow-hidden rounded-[1.5rem] border border-white/10 bg-[#11100d]">
                    {results.length > 0 ? (
                        <div className="divide-y divide-white/10">
                            {results.map((result) => (
                                <a
                                    key={`${result.section}-${result.path}`}
                                    href={result.path}
                                    className="block px-5 py-4 transition-colors hover:bg-white/[0.04]"
                                >
                                    <div className="flex items-start justify-between gap-4">
                                        <div className="min-w-0">
                                            <p className="truncate text-sm font-semibold text-white">
                                                {result.title}
                                            </p>
                                            <p className="mt-1 text-[11px] uppercase tracking-[0.18em] text-white/35">
                                                {result.section}
                                            </p>
                                        </div>
                                        <p className="shrink-0 text-xs text-white/42">{result.path}</p>
                                    </div>
                                </a>
                            ))}
                        </div>
                    ) : (
                        <div className="px-5 py-8 text-sm text-white">
                            {query ? 'No match found.' : 'Documentation, install, changelog, and legal pages are searchable here.'}
                        </div>
                    )}
                </div>
            </section>
        </div>
    );
}
