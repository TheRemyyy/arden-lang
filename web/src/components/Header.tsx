import { useState } from 'react';
import { Link } from 'react-router-dom';

export function Header() {
    const [isMenuOpen, setIsMenuOpen] = useState(false);

    return (
        <header className="fixed top-0 left-0 z-50 w-full border-b border-zinc-800 bg-[#0a0a0a]/90 backdrop-blur">
            <div className="mx-auto flex h-16 w-full max-w-6xl items-center justify-between px-6">
                <div className="flex items-center gap-4">
                    <Link to="/" className="inline-flex items-center gap-3 text-base font-semibold tracking-tight text-white" onClick={() => setIsMenuOpen(false)}>
                        <img src="/logo.png" alt="Arden logo" className="h-8 w-8 rounded-lg" />
                        <span>Arden</span>
                    </Link>
                </div>

                <nav className="hidden items-center gap-8 md:flex">
                    <Link to="/docs/overview" className="text-sm font-medium text-zinc-300 transition-colors hover:text-white">
                        Documentation
                    </Link>
                    <Link to="/docs/stdlib/overview" className="text-sm font-medium text-zinc-300 transition-colors hover:text-white">
                        Standard Library
                    </Link>
                    <Link to="/changelog" className="text-sm font-medium text-zinc-300 transition-colors hover:text-white">
                        Changelog
                    </Link>
                    <a href="https://github.com/TheRemyyy/arden" target="_blank" rel="noopener noreferrer" className="text-zinc-300 transition-colors hover:text-white">
                        <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 24 24" aria-hidden="true"><path fillRule="evenodd" d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z" clipRule="evenodd"></path></svg>
                    </a>
                </nav>

                <button
                    className="text-zinc-300 transition-colors hover:text-white md:hidden"
                    onClick={() => setIsMenuOpen(!isMenuOpen)}
                >
                    {isMenuOpen ? (
                        <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" /></svg>
                    ) : (
                        <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h16M4 18h16" /></svg>
                    )}
                </button>
            </div>

            {isMenuOpen && (
                <div className="fixed left-0 top-16 z-40 flex w-full flex-col gap-5 border-b border-zinc-800 bg-[#0a0a0a] p-6 shadow-2xl md:hidden">
                    <Link to="/docs/overview" className="text-base font-medium text-zinc-200 hover:text-white" onClick={() => setIsMenuOpen(false)}>
                        Documentation
                    </Link>
                    <Link to="/docs/stdlib/overview" className="text-base font-medium text-zinc-200 hover:text-white" onClick={() => setIsMenuOpen(false)}>
                        Standard Library
                    </Link>
                    <Link to="/changelog" className="text-base font-medium text-zinc-200 hover:text-white" onClick={() => setIsMenuOpen(false)}>
                        Changelog
                    </Link>
                    <a href="https://github.com/TheRemyyy/arden" target="_blank" rel="noopener noreferrer" className="flex items-center gap-2 text-base font-medium text-zinc-200 hover:text-white">
                        GitHub
                        <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 24 24" aria-hidden="true"><path fillRule="evenodd" d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z" clipRule="evenodd"></path></svg>
                    </a>
                </div>
            )}
        </header>
    );
}
