import ArrowRight from 'lucide-react/dist/esm/icons/arrow-right';
import Gauge from 'lucide-react/dist/esm/icons/gauge';
import MoveRight from 'lucide-react/dist/esm/icons/move-right';
import ShieldCheck from 'lucide-react/dist/esm/icons/shield-check';
import TerminalSquare from 'lucide-react/dist/esm/icons/terminal-square';
import { GITHUB_REPO_URL, RSS_FEED_SRC, SITE_CREATOR_URL } from '../lib/site';

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
            'Produce LLVM-backed native output through the same workflow surface you already used for checking, testing, and iteration.',
    },
];

const audienceCards = [
    {
        title: 'Systems developers',
        description:
            'Use Arden when you want a systems programming language with native software output, predictable tooling, and strong static checks.',
    },
    {
        title: 'Tooling-heavy teams',
        description:
            'Arden fits teams that care about developer workflow, command-line ergonomics, diagnostics, documentation, and repo-level consistency.',
    },
    {
        title: 'Performance-focused products',
        description:
            'It is a good fit for native utilities, internal developer tools, performance-sensitive services, and compiler-adjacent experiments.',
    },
];

const discoveryLinks = [
    {
        href: '/docs/overview',
        label: 'Read the documentation overview',
        description: 'Start with the language surface, projects, ownership, async support, and compiler workflow.',
    },
    {
        href: '/install',
        label: 'Open the installation guide',
        description: 'Download the latest portable bundle or follow the source build path for local development.',
    },
    {
        href: '/changelog',
        label: 'Browse release history',
        description: 'See how Arden evolves across compiler correctness, tooling, diagnostics, and project-system work.',
    },
    {
        href: GITHUB_REPO_URL,
        label: 'Inspect the repository',
        description: 'Review implementation details, examples, benchmarks, issues, and development history on GitHub.',
    },
    {
        href: RSS_FEED_SRC,
        label: 'Subscribe through RSS',
        description: 'Follow releases without relying on social platforms or third-party announcement feeds.',
    },
    {
        href: SITE_CREATOR_URL,
        label: 'Visit TheRemyyy',
        description: 'See the broader portfolio, linked projects, and the creator behind Arden on theremyyy.dev.',
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
                        <p className="mt-4 max-w-2xl text-base leading-8 text-[var(--text-muted)]">
                            This systems programming language is built for native software teams that want fast compiler feedback, readable ownership rules, project mode, and a workflow that stays coherent from the first file to larger repositories.
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

            <section className="content-auto-section border-y border-[rgba(57,52,46,0.12)] bg-[rgba(251,247,241,0.62)] py-20">
                <div className="mx-auto grid max-w-7xl gap-12 px-6 lg:grid-cols-[0.88fr_1.12fr]">
                    <div>
                        <p className="text-xs uppercase tracking-[0.24em] text-[var(--text-muted)]">
                            What Arden is
                        </p>
                        <h2 className="mt-4 max-w-lg font-display text-4xl font-bold leading-tight tracking-[-0.04em] md:text-5xl">
                            A systems programming language shaped around native software, fast workflow, and useful compiler feedback.
                        </h2>
                    </div>
                    <div className="grid gap-6 text-base leading-8 text-[var(--text-muted)]">
                        <p>
                            Arden is a systems programming language targeting LLVM and designed for people who care about native software, low-level control, and a workflow that does not fall apart once a project grows beyond a toy example. The core pitch is not abstract purity. It is practical compiler feedback, strong semantics, and a command-line surface that already knows about checking, testing, profiling, benchmarks, formatting, and project builds.
                        </p>
                        <p>
                            That matters because faster feedback changes how teams work. If the workflow is coherent, the compiler becomes a daily tool instead of a hurdle. If the diagnostics are readable, native software development gets easier to trust. If the project model is explicit, documentation, examples, and actual implementation stop drifting apart. Arden is trying to make those pieces feel like one product instead of disconnected tooling.
                        </p>
                    </div>
                </div>
            </section>

            <section className="content-auto-section mx-auto max-w-7xl px-6 py-20">
                <div className="grid gap-10 lg:grid-cols-[0.92fr_1.08fr]">
                    <div>
                        <p className="text-xs uppercase tracking-[0.24em] text-[var(--text-muted)]">
                            Workflow shape
                        </p>
                        <h2 className="mt-4 max-w-lg font-display text-4xl font-bold leading-tight tracking-[-0.04em] md:text-5xl">
                            The native software workflow should stay readable from first command to shipped build.
                        </h2>
                        <p className="mt-5 max-w-lg text-base leading-8 text-[var(--text-muted)]">
                            The homepage promise, documentation, and CLI need to agree with each other. That is why Arden keeps compiler feedback, project setup, testing, and build steps inside the same workflow language instead of scattering them across unrelated tools.
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
                                        Workflow
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

            <section className="content-auto-section border-y border-[rgba(57,52,46,0.12)] bg-[#1f1d1a] py-20 text-white">
                <div className="mx-auto max-w-7xl px-6">
                    <div className="max-w-3xl">
                        <p className="text-xs uppercase tracking-[0.24em] text-white/60">
                            Who Arden is for
                        </p>
                        <h2 className="mt-4 font-display text-4xl font-bold leading-tight tracking-[-0.04em] md:text-5xl">
                            Built for teams that want compiler rigor without a chaotic toolchain.
                        </h2>
                        <p className="mt-5 text-base leading-8 text-white/68">
                            Arden is relevant if you build native software, internal tools, CLI products, performance-sensitive services, language tooling, or systems-adjacent infrastructure and you care about readable code, predictable builds, and faster feedback loops.
                        </p>
                    </div>

                    <div className="mt-10 grid gap-4 md:grid-cols-3">
                        {audienceCards.map((card) => (
                            <article
                                key={card.title}
                                className="rounded-[1.75rem] border border-white/10 bg-white/[0.04] p-6"
                            >
                                <h3 className="text-2xl font-semibold tracking-[-0.03em] text-white">
                                    {card.title}
                                </h3>
                                <p className="mt-4 text-sm leading-7 text-white/68">
                                    {card.description}
                                </p>
                            </article>
                        ))}
                    </div>
                </div>
            </section>

            <section className="content-auto-section mx-auto max-w-7xl px-6 py-20">
                <div className="grid gap-10 lg:grid-cols-[0.82fr_1.18fr]">
                    <div>
                        <p className="text-xs uppercase tracking-[0.24em] text-[var(--text-muted)]">
                            Discover Arden
                        </p>
                        <h2 className="mt-4 max-w-lg font-display text-4xl font-bold leading-tight tracking-[-0.04em] md:text-5xl">
                            If there are no social channels, the project itself should still be easy to follow.
                        </h2>
                        <p className="mt-5 max-w-lg text-base leading-8 text-[var(--text-muted)]">
                            Arden does not need social plugins to be discoverable. The useful discovery surface is the docs, install path, changelog, repository, RSS feed, and the creator site that links the whole project graph together.
                        </p>
                    </div>

                    <div className="grid gap-4 sm:grid-cols-2">
                        {discoveryLinks.map((link) => (
                            <a
                                key={link.href}
                                href={link.href}
                                target={link.href.startsWith('http') ? '_blank' : undefined}
                                rel={link.href.startsWith('http') ? 'noreferrer' : undefined}
                                className="group rounded-[1.6rem] border border-[rgba(57,52,46,0.14)] bg-[rgba(251,247,241,0.84)] p-5 transition-all hover:-translate-y-0.5 hover:border-[var(--accent)]"
                            >
                                <div className="flex items-center justify-between gap-3">
                                    <h3 className="text-lg font-semibold tracking-[-0.02em] text-[var(--text)]">
                                        {link.label}
                                    </h3>
                                    <MoveRight className="h-4 w-4 text-[var(--accent)] transition-transform group-hover:translate-x-1" />
                                </div>
                                <p className="mt-3 text-sm leading-7 text-[var(--text-muted)]">
                                    {link.description}
                                </p>
                            </a>
                        ))}
                    </div>
                </div>
            </section>
        </div>
    );
}
