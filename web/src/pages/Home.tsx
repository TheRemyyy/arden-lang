import ArrowRight from 'lucide-react/dist/esm/icons/arrow-right';
import Gauge from 'lucide-react/dist/esm/icons/gauge';
import MoveRight from 'lucide-react/dist/esm/icons/move-right';
import ShieldCheck from 'lucide-react/dist/esm/icons/shield-check';
import TerminalSquare from 'lucide-react/dist/esm/icons/terminal-square';
import { GITHUB_REPO_URL } from '../lib/site';

const operatingPrinciples = [
    {
        title: 'Fast feedback loops',
        description:
            'Projects, tests, formatting, linting, profiling, and benchmarks are already part of the repo workflow.',
    },
    {
        title: 'Native output',
        description:
            'Arden targets LLVM directly and keeps the surface biased toward practical systems work over unnecessary abstraction.',
    },
    {
        title: 'Readable safety',
        description:
            'Ownership and static checks are there to prevent damage early, without turning every function into ceremony.',
    },
];

const capabilityRows = [
    {
        icon: ShieldCheck,
        title: 'Static checks that pull mistakes left',
        description:
            'Ownership, borrowing, mutability, and semantic validation push failures to compile time before they leak into runtime debugging.',
    },
    {
        icon: TerminalSquare,
        title: 'One CLI instead of scattered tooling',
        description:
            'Build, run, check, test, fmt, lint, fix, bench, profile, bindgen, parse, lex, and LSP support already sit in one workflow surface.',
    },
    {
        icon: Gauge,
        title: 'Project mode that goes beyond toy examples',
        description:
            'Multi-file builds, explicit source graphs, and reusable cache state make the repo feel like an actual language toolchain, not a parser demo.',
    },
];

export function Home() {
    return (
        <div className="overflow-x-hidden pt-16 text-[var(--text)]">
            <section className="site-grid relative min-h-[calc(100vh-4rem)] overflow-hidden">
                <div className="mx-auto grid max-w-7xl gap-10 px-6 pb-16 pt-10 lg:min-h-[calc(100vh-4rem)] lg:grid-cols-[1.12fr_0.88fr] lg:items-end lg:pb-20 lg:pt-14">
                    <div className="relative z-10">
                        <h1 className="max-w-4xl font-display text-5xl font-bold leading-[0.92] tracking-[-0.05em] text-[var(--text)] md:text-7xl">
                            Build native software with a sharper workflow and cleaner feedback.
                        </h1>
                        <p className="mt-5 max-w-2xl text-lg leading-8 text-[var(--text-muted)]">
                            Arden combines LLVM-backed native output, static safety checks, and an integrated command-line workflow so teams can move from experiments to multi-file projects without swapping mental models.
                        </p>
                        <div className="mt-8 flex flex-wrap gap-3">
                            <a
                                href="/docs/overview"
                                className="inline-flex h-12 items-center gap-2 rounded-full bg-[var(--bg-strong)] px-6 text-sm font-semibold text-white transition-transform hover:-translate-y-0.5"
                            >
                                Open documentation
                                <ArrowRight size={16} />
                            </a>
                            <a
                                href="/install"
                                className="inline-flex h-12 items-center rounded-full border border-[rgba(57,52,46,0.16)] bg-white/80 px-6 text-sm font-semibold text-[var(--text)] transition-colors hover:border-[var(--accent)] hover:text-[var(--accent)]"
                            >
                                Installation
                            </a>
                            <a
                                href="/docs/getting_started/quick_start"
                                className="inline-flex h-12 items-center rounded-full border border-[rgba(57,52,46,0.16)] bg-white/80 px-6 text-sm font-semibold text-[var(--text)] transition-colors hover:border-[var(--accent)] hover:text-[var(--accent)]"
                            >
                                Quick start
                            </a>
                        </div>
                    </div>

                    <div className="relative z-10">
                        <div className="overflow-hidden rounded-[2rem] border border-[rgba(57,52,46,0.14)] bg-[#1f1d1a] text-white shadow-[0_36px_80px_rgba(31,29,26,0.22)]">
                            <div className="border-b border-white/10 px-6 py-5">
                                <div>
                                    <p className="text-xs uppercase tracking-[0.24em] text-white/60">Repository-first workflow</p>
                                    <p className="mt-2 text-lg font-semibold">Fast path from zero to project mode</p>
                                </div>
                            </div>
                            <div className="grid gap-0 lg:grid-cols-[0.92fr_1.08fr]">
                                <div className="border-b border-white/10 bg-[#292621] px-6 py-6 lg:border-b-0 lg:border-r">
                                    <p className="text-xs uppercase tracking-[0.22em] text-white/60">Command flow</p>
                                    <pre className="mt-4 overflow-x-auto whitespace-pre-wrap text-sm leading-7 text-[#f5eee5]">
                                        <code>{`$ arden new radar\n$ cd radar\n$ arden check\n$ arden test\n$ arden run`}</code>
                                    </pre>
                                </div>
                                <div className="space-y-4 px-6 py-6">
                                    <p className="text-xs uppercase tracking-[0.22em] text-white/60">What this unlocks</p>
                                    <div className="grid gap-3">
                                        <div className="flex items-start justify-between gap-4 border-b border-white/8 pb-3">
                                            <span className="text-sm text-[#efe4d8]">LLVM-backed native code generation</span>
                                            <span className="text-xs uppercase tracking-[0.18em] text-[#d8b29e]">native</span>
                                        </div>
                                        <div className="flex items-start justify-between gap-4 border-b border-white/8 pb-3">
                                            <span className="text-sm text-[#efe4d8]">`arden.toml` project graphs and cache reuse</span>
                                            <span className="text-xs uppercase tracking-[0.18em] text-[#d8b29e]">project</span>
                                        </div>
                                        <div className="flex items-start justify-between gap-4">
                                            <span className="text-sm text-[#efe4d8]">Examples, docs, and benchmarks living in the same repo</span>
                                            <span className="text-xs uppercase tracking-[0.18em] text-[#d8b29e]">workflow</span>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </section>

            <section className="content-auto-section border-y border-[rgba(57,52,46,0.12)] bg-[#1f1d1a] py-8 text-white">
                <div className="mx-auto grid max-w-7xl gap-0 px-6 md:grid-cols-3">
                    {operatingPrinciples.map((principle, index) => (
                        <article
                            key={principle.title}
                            className={`py-6 md:px-8 ${index !== 0 ? 'md:border-l md:border-white/10' : ''}`}
                        >
                            <p className="text-xs uppercase tracking-[0.22em] text-white/60">
                                0{index + 1}
                            </p>
                            <h2 className="mt-4 text-2xl font-semibold tracking-[-0.03em] text-white">
                                {principle.title}
                            </h2>
                            <p className="mt-3 max-w-sm text-sm leading-7 text-[#d8cdc1]">
                                {principle.description}
                            </p>
                        </article>
                    ))}
                </div>
            </section>

            <section className="content-auto-section mx-auto max-w-7xl px-6 py-20">
                <div className="grid gap-10 lg:grid-cols-[0.8fr_1.2fr]">
                    <div>
                        <p className="text-xs uppercase tracking-[0.24em] text-[var(--text-muted)]">
                            Core capabilities
                        </p>
                        <h2 className="mt-4 max-w-md font-display text-4xl font-bold leading-tight tracking-[-0.04em] md:text-5xl">
                            The compiler, docs, and workflow should feel like one product.
                        </h2>
                        <p className="mt-5 max-w-md text-base leading-8 text-[var(--text-muted)]">
                            The repo is strongest when the language, examples, docs, and tooling reinforce each other instead of looking like separate side projects.
                        </p>
                        <a
                            href={GITHUB_REPO_URL}
                            target="_blank"
                            rel="noreferrer"
                            className="mt-8 inline-flex items-center gap-2 text-sm font-semibold text-[#b85c38] transition-colors hover:text-[var(--text)]"
                        >
                            Browse the repository
                            <MoveRight className="h-4 w-4" />
                        </a>
                    </div>

                    <div className="divide-y divide-[rgba(57,52,46,0.12)] border-y border-[rgba(57,52,46,0.12)]">
                        {capabilityRows.map((row) => {
                            const Icon = row.icon;

                            return (
                                <article key={row.title} className="grid gap-4 py-8 md:grid-cols-[56px_1fr] md:items-start">
                                    <div className="inline-flex h-14 w-14 items-center justify-center rounded-2xl bg-[var(--surface-soft)] text-[var(--accent)]">
                                        <Icon className="h-5 w-5" />
                                    </div>
                                    <div className="grid gap-3 md:grid-cols-[minmax(0,0.9fr)_minmax(0,1.1fr)]">
                                        <h3 className="text-2xl font-semibold tracking-[-0.03em] text-[var(--text)]">
                                            {row.title}
                                        </h3>
                                        <p className="text-sm leading-7 text-[var(--text-muted)]">
                                            {row.description}
                                        </p>
                                    </div>
                                </article>
                            );
                        })}
                    </div>
                </div>
            </section>
        </div>
    );
}
