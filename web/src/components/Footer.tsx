import { GITHUB_REPO_URL, SITE_NAME, UI_LOGO_SRC } from '../lib/site';

export function Footer() {
    return (
        <footer className="border-t border-[rgba(57,52,46,0.14)] bg-[#1f1d1a] py-16 text-[#d8cdc1]">
            <div className="mx-auto grid max-w-7xl gap-10 px-6 md:grid-cols-5">
                <div className="col-span-2">
                    <a href="/" className="mb-4 inline-flex items-center gap-3 text-lg font-semibold text-white">
                        <img
                            src={UI_LOGO_SRC}
                            alt={`${SITE_NAME} logo`}
                            width="36"
                            height="36"
                            decoding="async"
                            loading="lazy"
                            className="h-9 w-9 rounded-xl border border-white/10"
                        />
                        <span>{SITE_NAME}</span>
                    </a>
                    <p className="max-w-md text-sm leading-relaxed text-[#d8cdc1]">
                        Native systems programming with an opinionated toolchain, practical documentation, and a workflow that already includes projects, tests, formatting, linting, and benchmarks.
                    </p>
                </div>

                <div>
                    <h3 className="mb-4 text-sm font-semibold uppercase tracking-[0.18em] text-white/70">Resources</h3>
                    <ul className="space-y-3 text-sm">
                        <li><a href="/docs/overview" className="transition-colors hover:text-white">Docs Hub</a></li>
                        <li><a href="/install" className="transition-colors hover:text-white">Install Guide</a></li>
                        <li><a href="/docs/stdlib/overview" className="transition-colors hover:text-white">Stdlib Reference</a></li>
                        <li><a href="/docs/getting_started/quick_start" className="transition-colors hover:text-white">Quickstart Guide</a></li>
                        <li><a href="/changelog" className="transition-colors hover:text-white">Release Log</a></li>
                    </ul>
                </div>

                <div>
                    <h3 className="mb-4 text-sm font-semibold uppercase tracking-[0.18em] text-white/70">Project</h3>
                    <ul className="space-y-3 text-sm">
                        <li><a href={GITHUB_REPO_URL} className="transition-colors hover:text-white">GitHub Repository</a></li>
                        <li><a href="/docs/compiler/architecture" className="transition-colors hover:text-white">Compiler Architecture</a></li>
                        <li><a href="/docs/overview" className="transition-colors hover:text-white">Project Overview</a></li>
                    </ul>
                </div>

                <div>
                    <h3 className="mb-4 text-sm font-semibold uppercase tracking-[0.18em] text-white/70">Legal</h3>
                    <ul className="space-y-3 text-sm">
                        <li><a href="/terms" className="transition-colors hover:text-white">Terms of Use</a></li>
                        <li><a href="/privacy" className="transition-colors hover:text-white">Privacy Policy</a></li>
                    </ul>
                </div>
            </div>
            <div className="mx-auto mt-12 flex max-w-7xl flex-col items-center justify-between gap-3 border-t border-white/10 px-6 pt-8 text-center text-xs md:flex-row md:text-left">
                <p>&copy; {new Date().getFullYear()} {SITE_NAME}. Open source under Apache 2.0.</p>
                <a
                    href="https://www.theremyyy.dev/"
                    target="_blank"
                    rel="me noopener noreferrer"
                    className="transition-colors hover:text-white"
                >
                    TheRemyyy
                </a>
            </div>
        </footer>
    );
}
