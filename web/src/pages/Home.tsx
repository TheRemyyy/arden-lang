import ArrowRight from 'lucide-react/dist/esm/icons/arrow-right';
import Gauge from 'lucide-react/dist/esm/icons/gauge';
import MoveRight from 'lucide-react/dist/esm/icons/move-right';
import ShieldCheck from 'lucide-react/dist/esm/icons/shield-check';
import TerminalSquare from 'lucide-react/dist/esm/icons/terminal-square';
import { GITHUB_REPO_URL, SITE_CREATOR_URL } from '../lib/site';

const operatingPrinciples = [
    {
        title: 'Fast feedback loops',
        description:
            'Projects, tests, formatting, linting, profiling, and benchmarks are already part of the core command surface.',
    },
    {
        title: 'Native output',
        description:
            'Arden compiles to native binaries and keeps the language biased toward practical systems work over unnecessary abstraction.',
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
        title: 'Compile-time safety that pulls mistakes left',
        description:
            'Ownership, borrowing, mutability, and semantic validation push failures to compile time before they leak into runtime debugging.',
    },
    {
        icon: TerminalSquare,
        title: 'One CLI instead of scattered tooling',
        description:
            'Build, run, check, test, fmt, lint, fix, bench, profile, bindgen, parse, lex, and LSP support already sit behind one CLI surface.',
    },
    {
        icon: Gauge,
        title: 'Project mode that goes beyond toy examples',
        description:
            'Multi-file builds, explicit source graphs, and reusable cache state make the repo feel like an actual language toolchain, not a parser demo.',
    },
];

const workflowMoments = [
    {
        step: 'Create',
        command: 'arden new radar',
        description:
            'Start a native software project with a clean structure instead of hand-assembling files, build scripts, and ad-hoc conventions.',
    },
    {
        step: 'Validate',
        command: 'arden check',
        description:
            'Get fast compiler feedback on ownership, imports, mutability, and semantic issues before you waste time on runtime debugging.',
    },
    {
        step: 'Ship',
        command: 'arden build',
        description:
            'Produce release-ready binaries through the same command surface you already used for checking, testing, and iteration.',
    },
];

const audienceCards = [
    {
        title: 'Systems developers',
        description:
            'Use Arden when you want native binaries, predictable tooling, and strong compile-time safety without bolting together five separate layers.',
    },
    {
        title: 'Tooling-heavy teams',
        description:
            'It fits teams that care about command-line ergonomics, diagnostics, documentation, and a repo that behaves like one coherent product.',
    },
    {
        title: 'Performance-focused products',
        description:
            'It works well for native utilities, internal developer tools, performance-sensitive services, and compiler-adjacent experiments.',
    },
];

const discoveryLinks = [
    {
        href: '/docs/overview',
        label: 'Docs Overview',
        description: 'Start with the language surface, projects, ownership, async support, and the command model.',
    },
    {
        href: '/install',
        label: 'Install Arden',
        description: 'Download the latest portable bundle or follow the source build path for local development.',
    },
    {
        href: '/changelog',
        label: 'Release Notes',
        description: 'See how Arden evolves across correctness, diagnostics, developer experience, and project-system work.',
    },
    {
        href: GITHUB_REPO_URL,
        label: 'GitHub Repository',
        description: 'Review implementation details, examples, benchmarks, issues, and development history on GitHub.',
    },
    {
        href: SITE_CREATOR_URL,
        label: 'TheRemyyy',
        description: 'See the broader portfolio, linked projects, and the creator behind Arden on theremyyy.dev.',
    },
];

export function Home() {
    return (
        <div className="overflow-x-hidden pt-16 text-[var(--text)]">
            <section className="site-grid relative min-h-[calc(100vh-4rem)] overflow-hidden">
                <div className="mx-auto grid max-w-7xl gap-10 px-6 pb-16 pt-10 lg:min-h-[calc(100vh-4rem)] lg:grid-cols-[1.12fr_0.88fr] lg:items-end lg:pb-20 lg:pt-14">
                    <div className="relative z-10">
                        <h1 className="max-w-4xl font-display text-4xl font-bold leading-[0.95] tracking-[-0.04em] text-[var(--text)] md:text-6xl">
                            Arden is a native programming language with clearer compiler feedback.
                        </h1>
                        <p className="mt-5 max-w-2xl text-base leading-8 text-[var(--text-muted)] md:text-lg">
                            Build native binaries with a modern language surface, readable diagnostics, and practical tooling that stays fast as codebases grow.
                        </p>
                        <p className="mt-4 max-w-2xl text-sm leading-7 text-[var(--text-muted)]">
                            arden-lang keeps performance-first defaults without turning everyday development into ceremony.
                        </p>
                        <div className="mt-8 flex flex-wrap gap-3">
                            <a
                                href="/docs/overview"
                                className="inline-flex h-12 items-center gap-2 rounded-full bg-[var(--bg-strong)] px-6 text-sm font-semibold text-white transition-transform hover:-translate-y-0.5"
                            >
                                Docs overview
                                <ArrowRight size={16} />
                            </a>
                            <a
                                href="/install"
                                className="inline-flex h-12 items-center rounded-full border border-[rgba(57,52,46,0.16)] bg-white/80 px-6 text-sm font-semibold text-[var(--text)] transition-colors hover:border-[var(--accent)] hover:text-[var(--accent)]"
                            >
                                Install guide
                            </a>
                            <a
                                href="/docs/getting_started/quick_start"
                                className="inline-flex h-12 items-center rounded-full border border-[rgba(57,52,46,0.16)] bg-white/80 px-6 text-sm font-semibold text-[var(--text)] transition-colors hover:border-[var(--accent)] hover:text-[var(--accent)]"
                            >
                                5-minute quickstart
                            </a>
                        </div>
                    </div>

                    <div className="relative z-10">
                        <div className="overflow-hidden rounded-[2rem] border border-[rgba(57,52,46,0.14)] bg-[#1f1d1a] text-white shadow-[0_36px_80px_rgba(31,29,26,0.22)]">
                            <div className="border-b border-white/10 px-6 py-5">
                                <div>
                                    <p className="text-xs uppercase tracking-[0.24em] text-white/60">Repository-first setup</p>
                                    <p className="mt-2 text-lg font-semibold">Fast path from zero to multi-file projects</p>
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
                                            <span className="text-sm text-[#efe4d8]">Native code generation for real projects</span>
                                            <span className="text-xs uppercase tracking-[0.18em] text-[#d8b29e]">native</span>
                                        </div>
                                        <div className="flex items-start justify-between gap-4 border-b border-white/8 pb-3">
                                            <span className="text-sm text-[#efe4d8]">`arden.toml` project graphs and cache reuse</span>
                                            <span className="text-xs uppercase tracking-[0.18em] text-[#d8b29e]">project</span>
                                        </div>
                                        <div className="flex items-start justify-between gap-4">
                                            <span className="text-sm text-[#efe4d8]">Examples, docs, and benchmarks living in the same repo</span>
                                            <span className="text-xs uppercase tracking-[0.18em] text-[#d8b29e]">repo</span>
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
                            The compiler, docs, and commands should feel like one product.
                        </h2>
                        <p className="mt-5 max-w-md text-base leading-8 text-[var(--text-muted)]">
                            The repo is strongest when the language, examples, docs, and developer tools reinforce each other instead of looking like separate side projects.
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

            <section className="content-auto-section border-y border-[rgba(57,52,46,0.12)] bg-[rgba(251,247,241,0.62)] py-20 lg:min-h-screen lg:flex lg:items-center">
                <div className="mx-auto max-w-7xl px-6">
                    <div className="max-w-3xl">
                        <p className="text-xs uppercase tracking-[0.24em] text-[var(--text-muted)]">
                            What Arden is
                        </p>
                        <h2 className="mt-4 max-w-4xl font-display text-4xl font-bold leading-tight tracking-[-0.04em] md:text-5xl">
                            A systems programming language shaped around native software, practical commands, and useful compiler feedback.
                        </h2>
                    </div>
                    <div className="mt-10 max-w-5xl">
                        <p className="max-w-4xl font-display text-2xl italic leading-9 tracking-[-0.03em] text-[var(--text)] md:text-3xl">
                            Arden is for people who want multi-level control, readable semantics, and a toolchain that still feels clean once the codebase stops being tiny.
                        </p>
                        <div className="mt-8 grid gap-6 md:grid-cols-2">
                            <p className="text-sm leading-8 text-[var(--text-muted)]">
                                The pitch is not abstract purity. It is practical compiler feedback, strong semantics, and a command surface that already knows about checking, testing, profiling, benchmarks, formatting, and project builds. Instead of treating the language, docs, examples, and developer tools as separate side quests, Arden tries to keep them in the same orbit.
                            </p>
                            <p className="text-sm leading-8 text-[var(--text-muted)]">
                                That matters because faster feedback changes how teams work. If the commands stay coherent, the compiler becomes a daily tool instead of a hurdle. If the project model is explicit, documentation and implementation drift less. If diagnostics stay readable, lower-level development becomes easier to trust under real pressure.
                            </p>
                        </div>
                        <div className="mt-8 flex flex-wrap gap-x-8 gap-y-3 border-t border-[rgba(57,52,46,0.12)] pt-5 text-sm font-medium text-[var(--text-muted)]">
                            <span>Native builds</span>
                            <span>Ownership-aware semantics</span>
                            <span>Project mode</span>
                            <span>Readable diagnostics</span>
                            <span>Repo-level consistency</span>
                        </div>
                    </div>
                </div>
            </section>

            <section className="content-auto-section py-20 lg:min-h-screen lg:flex lg:items-center">
                <div className="mx-auto grid max-w-7xl gap-10 px-6 lg:grid-cols-[0.92fr_1.08fr]">
                    <div>
                        <p className="text-xs uppercase tracking-[0.24em] text-[var(--text-muted)]">
                            Workflow shape
                        </p>
                        <h2 className="mt-4 max-w-lg font-display text-4xl font-bold leading-tight tracking-[-0.04em] md:text-5xl">
                            The command surface should stay readable from first file to shipped build.
                        </h2>
                        <p className="mt-5 max-w-lg text-base leading-8 text-[var(--text-muted)]">
                            The homepage promise, documentation, and CLI need to agree with each other. That is why Arden keeps project setup, checking, testing, and build steps inside the same language instead of scattering them across unrelated tools.
                        </p>
                    </div>

                    <div className="grid gap-4">
                        {workflowMoments.map((moment) => (
                            <article
                                key={moment.step}
                                className="rounded-[1.75rem] border border-[rgba(57,52,46,0.14)] bg-[rgba(251,247,241,0.84)] p-6 shadow-[0_18px_60px_rgba(31,29,26,0.06)]"
                            >
                                <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
                                    <div>
                                        <p className="text-xs uppercase tracking-[0.22em] text-[var(--text-muted)]">
                                            {moment.step}
                                        </p>
                                        <h3 className="mt-2 text-2xl font-semibold tracking-[-0.03em] text-[var(--text)]">
                                            {moment.command}
                                        </h3>
                                    </div>
                                    <span className="rounded-full border border-[rgba(57,52,46,0.14)] bg-[var(--surface-soft)] px-3 py-1 text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--accent)]">
                                        CLI
                                    </span>
                                </div>
                                <p className="mt-4 text-sm leading-7 text-[var(--text-muted)]">
                                    {moment.description}
                                </p>
                            </article>
                        ))}
                    </div>
                </div>
            </section>

            <section className="content-auto-section border-y border-[rgba(57,52,46,0.12)] bg-[#1f1d1a] py-10 text-white">
                <div className="mx-auto max-w-7xl px-6">
                    <div className="grid gap-0 md:grid-cols-3">
                        {audienceCards.map((card, index) => (
                            <article
                                key={card.title}
                                className={`py-6 md:px-8 ${index !== 0 ? 'md:border-l md:border-white/10' : ''}`}
                            >
                                <p className="text-xs uppercase tracking-[0.22em] text-white/60">
                                    0{index + 1}
                                </p>
                                <h2 className="mt-4 text-2xl font-semibold tracking-[-0.03em] text-white">
                                    {card.title}
                                </h2>
                                <p className="mt-3 max-w-sm text-sm leading-7 text-[#d8cdc1]">
                                    {card.description}
                                </p>
                            </article>
                        ))}
                    </div>
                </div>
            </section>

            <section className="content-auto-section py-20 lg:min-h-screen lg:flex lg:items-center">
                <div className="mx-auto grid max-w-7xl gap-10 px-6 lg:grid-cols-[0.82fr_1.18fr]">
                    <div>
                        <p className="text-xs uppercase tracking-[0.24em] text-[var(--text-muted)]">
                            Discover Arden
                        </p>
                        <h2 className="mt-4 max-w-lg font-display text-4xl font-bold leading-tight tracking-[-0.04em] md:text-5xl">
                            If there are no social channels, the project itself should still be easy to follow.
                        </h2>
                        <p className="mt-5 max-w-lg text-base leading-8 text-[var(--text-muted)]">
                            Arden does not need social plugins to be discoverable. The useful discovery surface is the docs, install path, changelog, repository, and the creator site that links the whole project graph together.
                        </p>
                    </div>

                    <div className="space-y-5">
                        {discoveryLinks.map((link, index) => (
                            <article
                                key={link.href}
                                className="grid gap-3 border-b border-[rgba(57,52,46,0.14)] pb-5 last:border-b-0 sm:grid-cols-[minmax(0,0.78fr)_minmax(0,1.22fr)] sm:items-start"
                            >
                                <div className="flex items-center justify-between gap-3">
                                    <div>
                                        <p className="text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--text-muted)]">
                                            {String(index + 1).padStart(2, '0')}
                                        </p>
                                        <a
                                            href={link.href}
                                            target={link.href.startsWith('http') ? '_blank' : undefined}
                                            rel={link.href.startsWith('http') ? 'noreferrer' : undefined}
                                            className="group mt-2 inline-flex items-center gap-2 text-lg font-semibold tracking-[-0.02em] text-[var(--text)] transition-colors hover:text-[var(--accent)]"
                                        >
                                            {link.label}
                                            <MoveRight className="h-4 w-4 shrink-0 text-[var(--accent)] transition-transform group-hover:translate-x-1" />
                                        </a>
                                    </div>
                                </div>
                                <p className="text-sm leading-7 text-[var(--text-muted)]">
                                    {link.description}
                                </p>
                            </article>
                        ))}
                    </div>
                </div>
            </section>
        </div>
    );
}
