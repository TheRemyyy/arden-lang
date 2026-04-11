import Search from 'lucide-react/dist/esm/icons/search';
import X from 'lucide-react/dist/esm/icons/x';
import { useEffect, useMemo, useState } from 'react';
import { SITE_SEARCH_OPEN_EVENT } from '../lib/search-events';
import { searchSite } from '../lib/site-search';

export function SiteSearch() {
    const [isOpen, setIsOpen] = useState(false);
    const [query, setQuery] = useState('');

    const results = useMemo(() => searchSite(query), [query]);

    useEffect(() => {
        const onOpen = () => setIsOpen(true);
        const onKeyDown = (event: KeyboardEvent) => {
            const isShortcut = (event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k';
            if (isShortcut) {
                event.preventDefault();
                setIsOpen(true);
                return;
            }

            if (event.key === 'Escape') {
                setIsOpen(false);
            }
        };

        window.addEventListener(SITE_SEARCH_OPEN_EVENT, onOpen);
        window.addEventListener('keydown', onKeyDown);

        return () => {
            window.removeEventListener(SITE_SEARCH_OPEN_EVENT, onOpen);
            window.removeEventListener('keydown', onKeyDown);
        };
    }, []);

    useEffect(() => {
        if (!isOpen) {
            setQuery('');
        }
    }, [isOpen]);

    if (!isOpen) return null;

    return (
        <div
            className="fixed inset-0 z-[90] bg-[rgba(11,10,8,0.9)] px-4 py-20 backdrop-blur-sm"
            onClick={() => setIsOpen(false)}
        >
            <div
                className="mx-auto w-full max-w-2xl overflow-hidden rounded-[1.75rem] border border-white/10 bg-[#11100d] shadow-[0_40px_120px_rgba(0,0,0,0.58)]"
                onClick={(event) => event.stopPropagation()}
            >
                <div className="flex items-center gap-3 border-b border-white/10 px-4 py-4 sm:px-5">
                    <Search className="h-5 w-5 shrink-0 text-white/60" />
                    <input
                        autoFocus
                        value={query}
                        onChange={(event) => setQuery(event.target.value)}
                        type="text"
                        placeholder="Search docs, install, changelog..."
                        className="w-full appearance-none bg-transparent text-base text-white caret-white outline-none placeholder:text-white/35"
                    />
                    <button
                        type="button"
                        onClick={() => setIsOpen(false)}
                        className="inline-flex h-9 w-9 items-center justify-center rounded-full border border-white/10 bg-white/[0.04] text-white transition-colors hover:bg-white/[0.08]"
                        aria-label="Close search"
                    >
                        <X className="h-4 w-4 text-white" />
                    </button>
                </div>

                <div className="custom-scrollbar max-h-[65vh] overflow-y-auto py-2">
                    {results.length > 0 ? (
                        results.map((result) => (
                            <a
                                key={`${result.section}-${result.path}`}
                                href={result.path}
                                onClick={() => setIsOpen(false)}
                                className="block px-4 py-3 transition-colors hover:bg-white/[0.04] sm:px-5"
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
                        ))
                    ) : (
                        <div className="px-4 py-6 text-sm text-white sm:px-5">
                            No match found.
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
}
