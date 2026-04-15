import {
    Apple,
    Download,
    ExternalLink,
    GitBranch,
    MonitorDown,
    MonitorSmartphone,
    ShieldCheck,
    TerminalSquare,
} from 'lucide-react';
import { useEffect, useState } from 'react';
import { CURRENT_VERSION } from '../lib/site';
import {
    detectPreferredInstallTarget,
    fetchLatestReleaseSummary,
    getFallbackReleaseUrl,
    getLatestChecksumsDownloadUrl,
    getLatestDownloadUrl,
    getRecommendedInstallOption,
    INSTALL_DOCS_PATH,
    INSTALL_OPTIONS,
    type InstallOption,
    type InstallReleaseSummary,
    type InstallTargetId,
} from '../lib/install';

type InstallationExperienceProps = {
    compact?: boolean;
};

const installSteps = [
    {
        title: 'Download the portable bundle',
        description: 'Pick your platform and grab the latest stable archive straight from GitHub Releases.',
        icon: Download,
    },
    {
        title: 'Extract and launch Arden',
        description: 'Each bundle already includes Arden, LLVM, linker helpers, and the matching launcher script.',
        icon: MonitorDown,
    },
    {
        title: 'Verify and start building',
        description: 'Run `arden --version`, then jump into the quick start or source install docs if you want full repo setup.',
        icon: ShieldCheck,
    },
];

function formatPublishedDate(value: string | null): string | null {
    if (!value) {
        return null;
    }

    const parsedDate = new Date(value);
    if (Number.isNaN(parsedDate.valueOf())) {
        return null;
    }

    return new Intl.DateTimeFormat('en', {
        month: 'short',
        day: 'numeric',
        year: 'numeric',
    }).format(parsedDate);
}

function getPlatformIcon(option: InstallOption) {
    if (option.family === 'macos') {
        return Apple;
    }

    if (option.family === 'linux') {
        return TerminalSquare;
    }

    return MonitorSmartphone;
}

function DownloadActions({
    option,
    release,
    compact = false,
}: {
    option: InstallOption;
    release: InstallReleaseSummary | null;
    compact?: boolean;
}) {
    const sharedClassName = compact
        ? 'inline-flex h-11 items-center justify-center rounded-full px-5 text-sm font-semibold'
        : 'inline-flex h-12 items-center justify-center rounded-full px-6 text-sm font-semibold';
    const showChecksumsButton = !compact;

    return (
        <div className={`flex flex-wrap gap-3 ${compact ? '' : 'mt-6'}`}>
            <a
                href={getLatestDownloadUrl(option)}
                className={`${sharedClassName} bg-[var(--bg-strong)] text-white transition-transform hover:-translate-y-0.5`}
            >
                Download {option.label}
            </a>
            <a
                href={release?.releaseUrl ?? getFallbackReleaseUrl()}
                target="_blank"
                rel="noreferrer"
                className={`${sharedClassName} border border-[rgba(57,52,46,0.16)] bg-white/80 text-[var(--text)] transition-colors hover:border-[var(--accent)] hover:text-[var(--accent)]`}
            >
                View release
            </a>
            {showChecksumsButton && (
                <a
                    href={release?.checksumsUrl ?? getLatestChecksumsDownloadUrl()}
                    target="_blank"
                    rel="noreferrer"
                    className={`${sharedClassName} border border-[rgba(57,52,46,0.16)] bg-transparent text-[var(--text-muted)] transition-colors hover:border-[var(--accent)] hover:text-[var(--text)]`}
                >
                    Checksums
                </a>
            )}
        </div>
    );
}

function PlatformCard({
    option,
    release,
}: {
    option: InstallOption;
    release: InstallReleaseSummary | null;
}) {
    const isAssetKnown = release ? release.availableAssets.includes(option.assetName) : true;
    const PlatformIcon = getPlatformIcon(option);

    return (
        <article className="flex h-full flex-col justify-between rounded-[1.75rem] border border-[rgba(57,52,46,0.14)] bg-[rgba(251,247,241,0.84)] p-6 transition-colors">
            <div>
                <div className="flex items-start justify-between gap-4">
                    <div>
                        <p className="text-xs uppercase tracking-[0.22em] text-[var(--text-muted)]">
                            {option.family}
                        </p>
                        <div className="mt-3 inline-flex h-11 w-11 items-center justify-center rounded-2xl bg-[var(--surface-soft)] text-[var(--accent)]">
                            <PlatformIcon className="h-5 w-5" />
                        </div>
                        <h3 className="mt-3 whitespace-nowrap text-2xl font-semibold tracking-[-0.03em] text-[var(--text)]">
                            {option.label}
                        </h3>
                    </div>
                </div>
                <div className={`mt-5 h-2 rounded-full bg-gradient-to-r ${option.accentClass}`} />
                <p className="mt-5 text-sm leading-7 text-[var(--text-muted)]">
                    {option.summary}
                </p>
                <ul className="mt-5 space-y-2 text-sm text-[var(--text-muted)]">
                    <li>Archive: `{option.archiveType}` portable bundle</li>
                    <li>Launcher included: `arden` or `arden.cmd`</li>
                    <li>Status: {isAssetKnown ? 'published on the latest stable release' : 'waiting on the next release publish'}</li>
                </ul>
            </div>

            <div className="mt-8 flex flex-wrap gap-3">
                <a
                    href={getLatestDownloadUrl(option)}
                    className="inline-flex h-11 items-center justify-center rounded-full bg-[var(--bg-strong)] px-5 text-sm font-semibold text-white transition-transform hover:-translate-y-0.5"
                >
                    <Download className="mr-2 h-4 w-4" />
                    Download
                </a>
                <a
                    href={release?.releaseUrl ?? getFallbackReleaseUrl()}
                    target="_blank"
                    rel="noreferrer"
                    className="inline-flex h-11 items-center justify-center rounded-full border border-[rgba(57,52,46,0.16)] bg-white/80 px-5 text-sm font-semibold text-[var(--text)] transition-colors hover:border-[var(--accent)] hover:text-[var(--accent)]"
                >
                    Release notes
                </a>
                <a
                    href={release?.checksumsUrl ?? getLatestChecksumsDownloadUrl()}
                    target="_blank"
                    rel="noreferrer"
                    className="inline-flex h-11 items-center justify-center rounded-full border border-[rgba(57,52,46,0.12)] bg-transparent px-5 text-sm font-semibold text-[var(--text-muted)] transition-colors hover:border-[var(--accent)] hover:text-[var(--text)]"
                >
                    Checksums
                </a>
            </div>
        </article>
    );
}

export function InstallationExperience({ compact = false }: InstallationExperienceProps) {
    const [preferredTargetId, setPreferredTargetId] = useState<InstallTargetId | null>(() =>
        typeof window === 'undefined' ? null : detectPreferredInstallTarget(window.navigator),
    );
    const [releaseSummary, setReleaseSummary] = useState<InstallReleaseSummary | null>({
        versionLabel: CURRENT_VERSION,
        publishedAt: null,
        releaseUrl: getFallbackReleaseUrl(),
        checksumsUrl: getLatestChecksumsDownloadUrl(),
        availableAssets: [],
    });
    const [releaseState, setReleaseState] = useState<'idle' | 'ready' | 'failed'>('ready');

    useEffect(() => {
        setPreferredTargetId(detectPreferredInstallTarget(window.navigator));

        const controller = new AbortController();

        fetchLatestReleaseSummary(controller.signal)
            .then((summary) => {
                if (!controller.signal.aborted) {
                    setReleaseSummary(summary);
                    setReleaseState(summary ? 'ready' : 'failed');
                }
            })
            .catch(() => {
                if (!controller.signal.aborted) {
                    setReleaseState('failed');
                }
            });

        return () => controller.abort();
    }, []);

    const recommendedOption = getRecommendedInstallOption(preferredTargetId);
    const publishedDate = formatPublishedDate(releaseSummary?.publishedAt ?? null);

    if (compact) {
        return (
            <section className="paper-panel rounded-[2rem] p-8 md:p-10">
                <div className="grid gap-6 lg:grid-cols-[1fr_auto] lg:items-end">
                    <div>
                        <p className="text-xs uppercase tracking-[0.24em] text-[var(--text-muted)]">
                            Installation
                        </p>
                        <h2 className="mt-4 font-display text-4xl font-bold leading-tight tracking-[-0.04em] text-[var(--text)] md:text-5xl">
                            Install Arden in minutes, not after a dependency scavenger hunt.
                        </h2>
                        <p className="mt-5 max-w-2xl text-base leading-8 text-[var(--text-muted)]">
                            Grab the matching portable bundle, unpack it, run the included launcher, and move straight into compiling. The repo docs stay there if you want the deeper source-build path.
                        </p>
                    </div>
                </div>
                <div className="mt-8">
                    <DownloadActions option={recommendedOption} release={releaseSummary} compact />
                </div>
                <div className="mt-5 flex flex-wrap gap-4 text-sm text-[var(--text-muted)]">
                    <a href="/install" className="inline-flex items-center gap-2 font-semibold text-[var(--accent)] transition-colors hover:text-[var(--text)]">
                        Choose another platform
                        <ExternalLink className="h-4 w-4" />
                    </a>
                    <a
                        href={releaseSummary?.checksumsUrl ?? getLatestChecksumsDownloadUrl()}
                        target="_blank"
                        rel="noreferrer"
                        className="inline-flex items-center gap-2 transition-colors hover:text-[var(--text)]"
                    >
                        Download checksums
                        <ExternalLink className="h-4 w-4" />
                    </a>
                    <a href={INSTALL_DOCS_PATH} className="inline-flex items-center gap-2 transition-colors hover:text-[var(--text)]">
                        Source install guide
                        <GitBranch className="h-4 w-4" />
                    </a>
                </div>
            </section>
        );
    }

    return (
        <div className="mx-auto max-w-7xl px-6 pb-24 pt-24 md:pt-28">
            <section className="grid gap-10 pb-14 lg:grid-cols-[0.92fr_1.08fr] lg:items-end">
                <div>
                    <p className="text-xs uppercase tracking-[0.24em] text-[var(--text-muted)]">
                        Portable installation
                    </p>
                    <h1 className="mt-4 max-w-4xl font-display text-5xl font-bold leading-[0.94] tracking-[-0.05em] text-[var(--text)] md:text-7xl">
                        Download Arden and start compiling fast.
                    </h1>
                    <p className="mt-5 max-w-2xl text-lg leading-8 text-[var(--text-muted)]">
                        Choose the archive that matches your machine and you get Arden, the expected LLVM layout, linker helpers, and a launcher script in one shot. If you want to hack on the compiler itself, the source-build docs are still one click away.
                    </p>
                    <DownloadActions option={recommendedOption} release={releaseSummary} />
                </div>

                <div className="paper-panel rounded-[2rem] p-7">
                    <div className="flex items-center justify-between gap-6">
                        <div>
                            <p className="text-xs uppercase tracking-[0.22em] text-[var(--text-muted)]">Detected platform</p>
                            <p className="mt-3 text-3xl font-semibold tracking-[-0.03em] text-[var(--text)]">
                                {preferredTargetId ? recommendedOption.label : 'Choose manually'}
                            </p>
                        </div>
                        <div className="text-right">
                            <p className="text-xs uppercase tracking-[0.18em] text-[var(--text-muted)]">Release</p>
                            <p className="mt-2 text-lg font-semibold tracking-[-0.03em] text-[var(--text)]">
                                {releaseState === 'ready' ? releaseSummary?.versionLabel ?? 'Latest stable' : 'Latest stable'}
                            </p>
                        </div>
                    </div>
                    <p className="mt-4 text-sm leading-7 text-[var(--text-muted)]">
                        {releaseState === 'ready'
                            ? `${releaseSummary?.versionLabel ?? 'Latest stable'} is live${publishedDate ? `, published ${publishedDate}` : ''}.`
                            : 'Download buttons point to the latest stable GitHub release and keep working even if release metadata is temporarily unavailable.'}
                    </p>
                    <div className="mt-6 grid gap-3 text-sm text-[var(--text-muted)] sm:grid-cols-2">
                        <div className="rounded-[1.25rem] border border-[rgba(57,52,46,0.12)] bg-white/55 p-4">
                            <p className="font-semibold text-[var(--text)]">Included</p>
                            <p className="mt-2">Arden itself, the LLVM layout it expects, linker helpers, and the launcher wrapper that makes the bundle usable immediately.</p>
                        </div>
                        <div className="rounded-[1.25rem] border border-[rgba(57,52,46,0.12)] bg-white/55 p-4">
                            <p className="font-semibold text-[var(--text)]">Need the repo workflow?</p>
                            <p className="mt-2">Build from source only when you want compiler development, custom local toolchains, or CI-like setup.</p>
                        </div>
                    </div>
                    <div className="mt-6 flex flex-wrap gap-3 text-sm">
                        <a href={INSTALL_DOCS_PATH} className="inline-flex items-center gap-2 font-semibold text-[var(--accent)] transition-colors hover:text-[var(--text)]">
                            Source install docs
                            <GitBranch className="h-4 w-4" />
                        </a>
                        <a
                            href={releaseSummary?.checksumsUrl ?? releaseSummary?.releaseUrl ?? getFallbackReleaseUrl()}
                            target="_blank"
                            rel="noreferrer"
                            className="inline-flex items-center gap-2 text-[var(--text-muted)] transition-colors hover:text-[var(--text)]"
                        >
                            Open release metadata
                            <ExternalLink className="h-4 w-4" />
                        </a>
                    </div>
                </div>
            </section>

            <section className="border-t border-[rgba(57,52,46,0.12)] pt-16">
                <div className="grid gap-4 md:grid-cols-3">
                    {installSteps.map((step) => {
                        const Icon = step.icon;

                        return (
                            <article key={step.title} className="pt-5">
                                <div className="inline-flex h-12 w-12 items-center justify-center rounded-2xl bg-[var(--surface-soft)] text-[var(--accent)]">
                                    <Icon className="h-5 w-5" />
                                </div>
                                <h2 className="mt-5 text-2xl font-semibold tracking-[-0.03em] text-[var(--text)]">
                                    {step.title}
                                </h2>
                                <p className="mt-3 text-sm leading-7 text-[var(--text-muted)]">
                                    {step.description}
                                </p>
                            </article>
                        );
                    })}
                </div>
            </section>

            <section className="mt-16 border-t border-[rgba(57,52,46,0.12)] pt-16">
                <div className="flex items-end justify-between gap-6">
                    <div>
                        <p className="text-xs uppercase tracking-[0.24em] text-[var(--text-muted)]">
                            Downloads
                        </p>
                        <h2 className="mt-4 text-4xl font-bold tracking-[-0.04em] text-[var(--text)] md:text-5xl">
                            Portable bundles
                            <br />
                            for each supported platform.
                        </h2>
                        <p className="mt-4 max-w-2xl text-sm leading-7 text-[var(--text-muted)]">
                            Download the exact archive you need, verify it with checksums, and keep the source install path only for compiler work or custom toolchain setups.
                        </p>
                    </div>
                </div>
                <div className="mt-10 grid gap-5 xl:grid-cols-4 md:grid-cols-2">
                    {INSTALL_OPTIONS.map((option) => (
                        <PlatformCard
                            key={option.id}
                            option={option}
                            release={releaseSummary}
                        />
                    ))}
                </div>
            </section>
        </div>
    );
}
