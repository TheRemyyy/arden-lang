import Search from 'lucide-react/dist/esm/icons/search';
import { useState } from 'react';
import { openSiteSearch } from '../lib/search-events';
import { GITHUB_REPO_URL, SITE_NAME, UI_LOGO_SRC } from '../lib/site';

const navLinks = [
    { href: '/docs/overview', label: 'Documentation' },
    { href: '/install', label: 'Installation' },
    { href: '/docs/getting_started/quick_start', label: 'Quick Start' },
    { href: '/changelog', label: 'Changelog' },
];

export function Header() {
    const [isMenuOpen, setIsMenuOpen] = useState(false);

    return (
        <header className="fixed top-0 left-0 z-50 w-full border-b border-white/10 bg-[#1f1d1a]">
            <div className="mx-auto flex h-16 w-full max-w-7xl items-center justify-between px-6">
                <div className="flex items-center gap-4">
                    <a href="/" className="inline-flex items-center gap-3 text-base font-semibold tracking-tight text-white" onClick={() => setIsMenuOpen(false)}>
                        <img
                            src={UI_LOGO_SRC}
                            alt={`${SITE_NAME} logo`}
                            width="32"
                            height="32"
                            decoding="async"
                            fetchPriority="high"
                            className="h-8 w-8 rounded-xl border border-white/10"
                        />
                        <span>{SITE_NAME}</span>
                    </a>
                </div>

                <nav className="hidden items-center gap-7 md:flex">
                    {navLinks.map((link) => (
                        <a key={link.href} href={link.href} className="text-sm font-medium text-white/65 transition-colors hover:text-white">
                            {link.label}
                        </a>
                    ))}
                    <button
                        type="button"
                        onClick={() => openSiteSearch()}
                        className="inline-flex h-10 items-center gap-2 rounded-full border border-white/10 bg-white/5 px-4 text-sm font-medium text-white transition-colors hover:bg-white/10"
                    >
                        <Search className="h-4 w-4" />
                        Search
                        <span className="rounded-full border border-white/10 px-2 py-0.5 text-[10px] uppercase tracking-[0.18em] text-white/42">
                            Ctrl K
                        </span>
                    </button>
                    <a
                        href={GITHUB_REPO_URL}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="inline-flex h-10 items-center gap-2 rounded-full border border-white/10 bg-white/5 px-4 text-sm font-medium text-white transition-colors hover:bg-white/10"
                    >
                        GitHub Repo
                        <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 24 24" aria-hidden="true"><path fillRule="evenodd" d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z" clipRule="evenodd"></path></svg>
                    </a>
                </nav>

                <div className="flex items-center gap-2 md:hidden">
                    <button
                        type="button"
                        className="inline-flex h-10 items-center justify-center gap-2 rounded-full border border-white/10 bg-white/5 px-4 text-sm font-medium text-white transition-colors hover:bg-white/10"
                        onClick={() => openSiteSearch()}
                        aria-label="Open search"
                    >
                        <Search className="h-4 w-4" />
                        Search
                    </button>
                    <button
                        type="button"
                        className="text-white/65 transition-colors hover:text-white"
                        onClick={() => setIsMenuOpen(!isMenuOpen)}
                        aria-label={isMenuOpen ? 'Close navigation menu' : 'Open navigation menu'}
                    >
                        {isMenuOpen ? (
                            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" /></svg>
                        ) : (
                            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h16M4 18h16" /></svg>
                        )}
                    </button>
                </div>
            </div>

            {isMenuOpen && (
                <div className="fixed left-0 top-16 z-40 flex w-full flex-col gap-5 border-b border-white/10 bg-[#1f1d1a] p-6 shadow-[0_24px_60px_rgba(0,0,0,0.35)] md:hidden">
                    {navLinks.map((link) => (
                        <a key={link.href} href={link.href} className="text-base font-medium text-white hover:text-[var(--accent-soft)]" onClick={() => setIsMenuOpen(false)}>
                            {link.label}
                        </a>
                    ))}
                    <a href={GITHUB_REPO_URL} target="_blank" rel="noopener noreferrer" className="flex items-center gap-2 text-base font-medium text-white hover:text-[var(--accent-soft)]">
                        GitHub Project
                        <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 24 24" aria-hidden="true"><path fillRule="evenodd" d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z" clipRule="evenodd"></path></svg>
                    </a>
                </div>
            )}
        </header>
    );
}
