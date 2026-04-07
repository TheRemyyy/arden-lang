import { GITHUB_REPO_URL, UI_LOGO_SRC } from '../lib/site';

export function Footer() {
    return (
        <footer className="border-t border-zinc-800 bg-[#0a0a0a] py-14 text-zinc-400">
            <div className="mx-auto grid max-w-6xl gap-10 px-6 md:grid-cols-4">
                <div className="col-span-2">
                    <a href="/" className="mb-4 inline-flex items-center gap-3 text-lg font-semibold text-white">
                        <img
                            src={UI_LOGO_SRC}
                            alt="Arden logo"
                            width="36"
                            height="36"
                            decoding="async"
                            className="h-9 w-9 rounded-xl"
                        />
                        <span>Arden</span>
                    </a>
                    <p className="max-w-sm text-sm leading-relaxed">
                        A modern systems programming language designed for reliability, performance, and developer ergonomics.
                    </p>
                </div>

                <div>
                    <h3 className="mb-4 text-sm font-semibold text-white">Resources</h3>
                    <ul className="space-y-3 text-sm">
                        <li><a href="/docs/overview" className="hover:text-white transition-colors">Documentation</a></li>
                        <li><a href="/docs/stdlib/overview" className="hover:text-white transition-colors">Standard Library</a></li>
                        <li><a href="/changelog" className="hover:text-white transition-colors">Changelog</a></li>
                        <li><a href={GITHUB_REPO_URL} className="hover:text-white transition-colors">GitHub</a></li>
                    </ul>
                </div>

                <div>
                    <h3 className="mb-4 text-sm font-semibold text-white">Community</h3>
                    <ul className="space-y-3 text-sm">
                        <li><a href="#" className="hover:text-white transition-colors">Discord (Coming Soon)</a></li>
                        <li><a href="#" className="hover:text-white transition-colors">Twitter (Coming Soon)</a></li>
                    </ul>
                </div>
            </div>
            <div className="mx-auto mt-12 flex max-w-6xl flex-col items-center justify-between gap-3 border-t border-zinc-800 px-6 pt-8 text-center text-xs md:flex-row md:text-left">
                <p>&copy; {new Date().getFullYear()} Arden. Open Source - Apache 2.0</p>
                <a
                    href="https://theremyyy.dev"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-zinc-400 transition-colors hover:text-zinc-200"
                >
                    TheRemyyy
                </a>
            </div>
        </footer>
    );
}
