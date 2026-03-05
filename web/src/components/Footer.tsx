import { Link } from 'react-router-dom';

export function Footer() {
    return (
        <footer className="border-t border-zinc-800 bg-[#0a0a0a] py-14 text-zinc-400">
            <div className="mx-auto grid max-w-6xl gap-10 px-6 md:grid-cols-4">
                <div className="col-span-2">
                    <Link to="/" className="mb-4 inline-flex items-center gap-2 text-lg font-semibold text-white">
                        Apex
                    </Link>
                    <p className="max-w-sm text-sm leading-relaxed">
                        A modern systems programming language designed for reliability, performance, and developer ergonomics.
                    </p>
                </div>

                <div>
                    <h3 className="mb-4 text-sm font-semibold text-white">Resources</h3>
                    <ul className="space-y-3 text-sm">
                        <li><Link to="/docs/overview.md" className="hover:text-white transition-colors">Documentation</Link></li>
                        <li><Link to="/docs/stdlib/overview.md" className="hover:text-white transition-colors">Standard Library</Link></li>
                        <li><Link to="/changelog" className="hover:text-white transition-colors">Changelog</Link></li>
                        <li><a href="https://github.com/TheRemyyy/apex-compiler" className="hover:text-white transition-colors">GitHub</a></li>
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
                <p>&copy; {new Date().getFullYear()} Apex Compiler. Open Source (MIT).</p>
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
