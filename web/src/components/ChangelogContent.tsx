import { useEffect, useRef, useState, type RefObject } from 'react';
import { motion } from 'framer-motion';
import { CreatorAttribution } from './CreatorAttribution';
import type { ChangelogRelease } from '../lib/changelog';

const categoryThemes: Record<string, string> = {
    added: 'border-emerald-500/20 bg-emerald-500/10 text-emerald-200',
    'new features': 'border-emerald-500/20 bg-emerald-500/10 text-emerald-200',
    changed: 'border-amber-500/20 bg-amber-500/10 text-amber-200',
    technical: 'border-sky-500/20 bg-sky-500/10 text-sky-200',
    fixed: 'border-rose-500/20 bg-rose-500/10 text-rose-200',
    'bug fixes': 'border-rose-500/20 bg-rose-500/10 text-rose-200',
    documentation: 'border-violet-500/20 bg-violet-500/10 text-violet-200',
    benchmarks: 'border-cyan-500/20 bg-cyan-500/10 text-cyan-200',
    'performance & optimization': 'border-cyan-500/20 bg-cyan-500/10 text-cyan-200',
    configuration: 'border-orange-500/20 bg-orange-500/10 text-orange-200',
    'major release: complete project system': 'border-yellow-500/20 bg-yellow-500/10 text-yellow-100',
    'major changes': 'border-yellow-500/20 bg-yellow-500/10 text-yellow-100',
    'behavior changes': 'border-orange-500/20 bg-orange-500/10 text-orange-200',
    'code refactoring': 'border-slate-500/20 bg-slate-400/10 text-slate-200',
};

function formatDate(date: string | null): string | null {
    if (!date) return null;
    return new Intl.DateTimeFormat('en', {
        year: 'numeric',
        month: 'short',
        day: 'numeric',
    }).format(new Date(`${date}T00:00:00Z`));
}

function getCategoryTheme(kind: string): string {
    return categoryThemes[kind] ?? 'border-white/10 bg-white/[0.04] text-white/82';
}

function getReleaseItemCount(release: ChangelogRelease): number {
    return release.categories.reduce((total, category) => total + category.itemCount, 0);
}

function estimateCategoryWeight(category: ChangelogRelease['categories'][number]): number {
    const htmlLengthWeight = Math.ceil(category.html.length / 1400);
    const itemWeight = Math.max(category.itemCount, 1);
    return itemWeight + htmlLengthWeight;
}

function shouldCollapseCategory(
    category: ChangelogRelease['categories'][number],
): boolean {
    if (category.itemCount >= 3) {
        return true;
    }

    return category.html.length > 3200;
}

function splitCategoriesIntoColumns(categories: ChangelogRelease['categories']) {
    const leftColumn: ChangelogRelease['categories'] = [];
    const rightColumn: ChangelogRelease['categories'] = [];
    let leftWeight = 0;
    let rightWeight = 0;

    categories.forEach((category) => {
        const categoryWeight = estimateCategoryWeight(category);
        if (leftWeight <= rightWeight) {
            leftColumn.push(category);
            leftWeight += categoryWeight;
            return;
        }

        rightColumn.push(category);
        rightWeight += categoryWeight;
    });

    return [leftColumn, rightColumn] as const;
}

function ReleaseSidebar({
    releases,
    activeReleaseId,
    boundaryRef,
}: {
    releases: ChangelogRelease[];
    activeReleaseId: string;
    boundaryRef: RefObject<HTMLDivElement | null>;
}) {
    const containerRef = useRef<HTMLDivElement | null>(null);
    const railRef = useRef<HTMLDivElement | null>(null);
    const [isDockedToBottom, setIsDockedToBottom] = useState(false);
    const [railWidth, setRailWidth] = useState(260);
    const [railHeight, setRailHeight] = useState(0);
    const [railLeft, setRailLeft] = useState<number | null>(null);

    useEffect(() => {
        let frameId = 0;
        const syncPosition = () => {
            const boundary = boundaryRef.current;
            const rail = railRef.current;
            const container = containerRef.current;
            if (!boundary || !rail || !container) return;

            const boundaryRect = boundary.getBoundingClientRect();
            const railRect = rail.getBoundingClientRect();
            const containerHeight = container.offsetHeight;
            const topOffset = 96;
            const nextDocked = boundaryRect.bottom <= topOffset + containerHeight;

            setIsDockedToBottom((current) => (current === nextDocked ? current : nextDocked));

            const nextWidth = Math.round(railRect.width);
            setRailWidth((current) => (current === nextWidth ? current : nextWidth));
            const nextHeight = Math.max(Math.round(boundary.offsetHeight), 0);
            setRailHeight((current) => (current === nextHeight ? current : nextHeight));
            const nextLeft = Math.round(railRect.left);
            setRailLeft((current) => (current === nextLeft ? current : nextLeft));
        };

        const requestSync = () => {
            if (frameId !== 0) return;
            frameId = window.requestAnimationFrame(() => {
                frameId = 0;
                syncPosition();
            });
        };

        syncPosition();
        window.addEventListener('scroll', requestSync, { passive: true });
        window.addEventListener('resize', requestSync);
        return () => {
            if (frameId !== 0) {
                window.cancelAnimationFrame(frameId);
            }
            window.removeEventListener('scroll', requestSync);
            window.removeEventListener('resize', requestSync);
        };
    }, [boundaryRef]);

    useEffect(() => {
        const container = containerRef.current;
        if (!container || !activeReleaseId) return;

        const activeLink = container.querySelector<HTMLElement>(`[data-release-id="${activeReleaseId}"]`);
        if (!activeLink) return;

        const containerTop = container.scrollTop;
        const containerBottom = containerTop + container.clientHeight;
        const itemTop = activeLink.offsetTop;
        const itemBottom = itemTop + activeLink.offsetHeight;
        const padding = 24;

        if (itemTop < containerTop + padding) {
            container.scrollTo({
                top: Math.max(itemTop - padding, 0),
                behavior: 'smooth',
            });
            return;
        }

        if (itemBottom > containerBottom - padding) {
            container.scrollTo({
                top: itemBottom - container.clientHeight + padding,
                behavior: 'smooth',
            });
        }
    }, [activeReleaseId]);

    return (
        <aside
            ref={railRef}
            className="relative hidden lg:block lg:w-[260px] lg:self-start"
            style={railHeight > 0 ? { minHeight: `${railHeight}px` } : undefined}
        >
            <div className="h-full min-h-[1px]">
                <div
                    ref={containerRef}
                    className={`custom-scrollbar max-h-[calc(100vh-7rem)] overflow-y-auto rounded-[2rem] border border-white/10 bg-white/[0.04] p-5 ${
                        isDockedToBottom ? 'absolute bottom-0 left-0' : 'fixed top-24'
                    }`}
                    style={
                        isDockedToBottom
                            ? { width: `${railWidth}px` }
                            : railLeft === null
                              ? { visibility: 'hidden', width: `${railWidth}px` }
                              : { left: `${railLeft}px`, width: `${railWidth}px` }
                    }
                >
                    <p className="text-xs font-semibold uppercase tracking-[0.22em] text-white/45">Versions</p>
                    <div className="mt-5 space-y-2">
                        {releases.map((release) => (
                            <a
                                key={release.id}
                                data-release-id={release.id}
                                href={`#${release.id}`}
                                className={`block rounded-2xl border px-3 py-3 transition-colors ${
                                    activeReleaseId === release.id
                                        ? 'border-[var(--accent-soft)] bg-white/[0.07]'
                                        : 'border-transparent hover:border-white/10 hover:bg-white/[0.04]'
                                }`}
                            >
                                <p className={`text-sm font-semibold ${activeReleaseId === release.id ? 'text-white' : 'text-white/82'}`}>
                                    {release.label}
                                </p>
                                {(release.subtitle || release.date) && (
                                    <p className={`mt-1 text-xs leading-5 ${activeReleaseId === release.id ? 'text-white/60' : 'text-white/45'}`}>
                                        {[release.subtitle, formatDate(release.date)].filter(Boolean).join(' · ')}
                                    </p>
                                )}
                                <p className={`mt-1 text-[11px] uppercase tracking-[0.16em] ${activeReleaseId === release.id ? 'text-[var(--accent-soft)]' : 'text-white/35'}`}>
                                    {getReleaseItemCount(release)} items
                                </p>
                            </a>
                        ))}
                    </div>
                </div>
            </div>
        </aside>
    );
}

function MobileReleaseRail({
    releases,
    activeReleaseId,
}: {
    releases: ChangelogRelease[];
    activeReleaseId: string;
}) {
    const containerRef = useRef<HTMLDivElement | null>(null);

    useEffect(() => {
        const container = containerRef.current;
        if (!container || !activeReleaseId) return;

        const activeLink = container.querySelector<HTMLElement>(`[data-release-id="${activeReleaseId}"]`);
        if (!activeLink) return;

        const containerLeft = container.scrollLeft;
        const containerRight = containerLeft + container.clientWidth;
        const itemLeft = activeLink.offsetLeft;
        const itemRight = itemLeft + activeLink.offsetWidth;
        const padding = 20;

        if (itemLeft < containerLeft + padding) {
            container.scrollTo({
                left: Math.max(itemLeft - padding, 0),
                behavior: 'smooth',
            });
            return;
        }

        if (itemRight > containerRight - padding) {
            container.scrollTo({
                left: itemRight - container.clientWidth + padding,
                behavior: 'smooth',
            });
        }
    }, [activeReleaseId]);

    return (
        <div className="fixed left-0 right-0 top-16 z-30 border-b border-white/10 bg-[#1f1d1a] lg:hidden">
            <div ref={containerRef} className="custom-scrollbar overflow-x-auto px-4 py-3">
                <div className="flex min-w-max gap-2">
                    {releases.map((release) => (
                        <a
                            key={release.id}
                            data-release-id={release.id}
                            href={`#${release.id}`}
                            className={`rounded-full border px-3 py-2 text-xs font-semibold uppercase tracking-[0.16em] transition-colors ${
                                activeReleaseId === release.id
                                    ? 'border-[var(--accent-soft)] bg-white/[0.08] text-white'
                                    : 'border-white/10 bg-white/[0.03] text-white/58'
                            }`}
                        >
                            {release.label}
                        </a>
                    ))}
                </div>
            </div>
        </div>
    );
}

function CategoryCard({
    category,
}: {
    category: ChangelogRelease['categories'][number];
}) {
    const shouldCollapseByDefault = shouldCollapseCategory(category);
    const [isExpanded, setIsExpanded] = useState(!shouldCollapseByDefault);

    return (
        <section className="mb-4 min-w-0 break-inside-avoid self-start overflow-hidden rounded-[1.35rem] border border-white/10 bg-white/[0.04] p-4 sm:p-5">
            <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between sm:gap-4">
                <div>
                    <h3 className="font-display text-xl font-semibold tracking-[-0.03em] text-white sm:text-2xl">
                        {category.plainTitle}
                    </h3>
                    {shouldCollapseByDefault && (
                        <p className="mt-2 text-sm leading-6 text-white/45">
                            Large section hidden by default to keep the page readable.
                        </p>
                    )}
                </div>
                <div className="flex flex-wrap items-center gap-2">
                    {category.itemCount > 0 && (
                        <span className="rounded-full border border-white/10 bg-white/[0.04] px-2.5 py-1 text-[11px] uppercase tracking-[0.16em] text-white/52">
                            {category.itemCount} items
                        </span>
                    )}
                    {shouldCollapseByDefault && (
                        <button
                            type="button"
                            onClick={() => setIsExpanded((current) => !current)}
                            className="inline-flex h-9 items-center rounded-full border border-white/10 bg-white/[0.04] px-3 text-[11px] font-semibold uppercase tracking-[0.16em] text-white/72 transition-colors hover:bg-white/[0.08]"
                        >
                            {isExpanded ? 'Collapse' : 'Expand'}
                        </button>
                    )}
                </div>
            </div>
            <div className={`relative mt-5 ${isExpanded ? '' : 'max-h-80 overflow-hidden'}`}>
                <article
                    className="prose prose-invert prose-zinc max-w-none break-words
                        prose-p:text-[14px] prose-p:leading-7 prose-p:text-white/72 sm:prose-p:text-[15px]
                        prose-ul:my-4 prose-ul:list-disc prose-ul:pl-5
                        prose-li:mb-2 prose-li:text-white/72
                        prose-strong:text-white
                        prose-a:text-[var(--accent-soft)] prose-a:no-underline hover:prose-a:text-white
                        prose-code:border-0 prose-code:bg-transparent prose-code:px-0 prose-code:py-0 prose-code:text-[13px] prose-code:text-[#f2d6c8] prose-code:before:content-none prose-code:after:content-none
                        prose-pre:rounded-[1.25rem] prose-pre:border prose-pre:border-white/10 prose-pre:bg-[#292621] prose-pre:text-[#f7efe5]
                        prose-blockquote:border-l-[var(--accent-soft)] prose-blockquote:text-white"
                    dangerouslySetInnerHTML={{ __html: category.html }}
                />
            </div>
            {shouldCollapseByDefault && (
                <div className="mt-4">
                    <button
                        type="button"
                        onClick={() => setIsExpanded((current) => !current)}
                        className="inline-flex h-9 items-center rounded-full border border-white/10 bg-white/[0.04] px-3 text-[11px] font-semibold uppercase tracking-[0.16em] text-white/72 transition-colors hover:bg-white/[0.08]"
                    >
                        {isExpanded ? 'Show less' : 'Show more'}
                    </button>
                </div>
            )}
        </section>
    );
}

function ReleaseCard({ release, index }: { release: ChangelogRelease; index: number }) {
    const [leftColumn, rightColumn] = splitCategoriesIntoColumns(release.categories);

    return (
        <motion.section
            id={release.id}
            initial={{ opacity: 0, y: 14 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, margin: '-80px' }}
            transition={{ duration: 0.25, delay: Math.min(index * 0.03, 0.16) }}
            className="min-w-0 scroll-mt-24 border-t border-white/10 py-8 first:border-t-0 first:pt-0 sm:py-10"
        >
            <div className="flex flex-col gap-4 border-b border-white/10 pb-5 md:flex-row md:items-end md:justify-between md:gap-5 md:pb-6">
                <div>
                    <p className="text-xs font-semibold uppercase tracking-[0.24em] text-[var(--accent-soft)]">
                        {release.label}
                    </p>
                    <h2 className="mt-3 font-display text-2xl font-bold tracking-[-0.04em] text-white sm:text-3xl md:text-4xl">
                        {release.displayTitle}
                    </h2>
                    {release.date && (
                        <p className="mt-3 text-sm uppercase tracking-[0.18em] text-white/45">
                            {formatDate(release.date)}
                        </p>
                    )}
                </div>
                <div className="flex flex-wrap gap-2">
                    {release.categories.map((category) => (
                        <span
                            key={category.id}
                            className={`rounded-full border px-3 py-1 text-[11px] font-semibold uppercase tracking-[0.18em] ${getCategoryTheme(category.kind)}`}
                        >
                            {category.plainTitle}
                        </span>
                    ))}
                </div>
            </div>

            {release.summaryHtml && (
                <article
                    className="prose prose-invert prose-zinc mt-5 max-w-none prose-p:text-[14px] prose-p:leading-7 prose-p:text-white/70 sm:prose-p:text-[15px] prose-a:text-[var(--accent-soft)] prose-a:no-underline hover:prose-a:text-white"
                    dangerouslySetInnerHTML={{ __html: release.summaryHtml }}
                />
            )}

            <div className="mt-6 xl:grid xl:grid-cols-2 xl:gap-5">
                <div>
                    {leftColumn.map((category) => (
                    <CategoryCard
                        key={category.id}
                        category={category}
                    />
                    ))}
                </div>
                <div>
                    {rightColumn.map((category) => (
                    <CategoryCard
                        key={category.id}
                        category={category}
                    />
                    ))}
                </div>
            </div>
        </motion.section>
    );
}

export function ChangelogContent({ releases }: { releases: ChangelogRelease[] }) {
    const [activeReleaseId, setActiveReleaseId] = useState(releases[0]?.id ?? '');
    const boundaryRef = useRef<HTMLDivElement | null>(null);

    useEffect(() => {
        if (releases.length === 0) return;
        const topOffset = 152;

        const updateActiveRelease = () => {
            const elements = releases
                .map((release) => document.getElementById(release.id))
                .filter((element): element is HTMLElement => element !== null);

            if (elements.length === 0) {
                return;
            }

            let currentActiveId = elements[0].id;

            for (const element of elements) {
                if (element.getBoundingClientRect().top <= topOffset) {
                    currentActiveId = element.id;
                } else {
                    break;
                }
            }

            setActiveReleaseId((current) => (current === currentActiveId ? current : currentActiveId));
        };

        updateActiveRelease();
        window.addEventListener('scroll', updateActiveRelease, { passive: true });
        window.addEventListener('resize', updateActiveRelease);
        return () => {
            window.removeEventListener('scroll', updateActiveRelease);
            window.removeEventListener('resize', updateActiveRelease);
        };
    }, [releases]);

    return (
        <div className="min-h-screen overflow-x-hidden bg-[#0f0d0b] pt-16 text-[#f3ece3]">
            <MobileReleaseRail releases={releases} activeReleaseId={activeReleaseId} />

            <div className="w-full px-3 pb-16 pt-24 sm:px-4 sm:pb-20 lg:px-4 xl:px-5">
                <div className="lg:grid lg:grid-cols-[260px_minmax(0,1fr)] lg:gap-6">
                    <div className="hidden lg:block" />
                    <div className="min-w-0">
                        <div className="mx-auto max-w-3xl text-center">
                            <p className="text-xs font-semibold uppercase tracking-[0.22em] text-[var(--accent-soft)]">
                                Release history
                            </p>
                            <h1 className="mt-5 font-display text-4xl font-bold tracking-[-0.04em] text-white sm:text-5xl md:text-6xl">
                                Changelog
                            </h1>
                            <p className="mx-auto mt-5 max-w-2xl text-base leading-7 text-white/68 sm:text-lg sm:leading-8">
                                Browse Arden releases by version and jump straight to added features, fixes, behavioral changes, and technical work.
                            </p>
                            <div className="mt-8 flex flex-wrap items-center justify-center gap-3">
                                <div className="rounded-full border border-white/10 bg-white/[0.04] px-4 py-2 text-sm text-white/72">
                                    {releases.length} tracked releases
                                </div>
                                <div className="rounded-full border border-white/10 bg-white/[0.04] px-4 py-2 text-sm text-white/72">
                                    {releases[0]?.label ?? 'Latest'} on top
                                </div>
                            </div>
                        </div>
                    </div>
                </div>

                <div
                    ref={boundaryRef}
                    className="relative mt-8 min-w-0 gap-6 lg:mt-14 lg:grid lg:grid-cols-[260px_minmax(0,1fr)] lg:items-start lg:gap-6"
                >
                    <ReleaseSidebar releases={releases} activeReleaseId={activeReleaseId} boundaryRef={boundaryRef} />
                    <div className="min-w-0 space-y-8">
                        {releases.map((release, index) => (
                            <ReleaseCard key={release.id} release={release} index={index} />
                        ))}
                        <CreatorAttribution />
                    </div>
                </div>
            </div>
        </div>
    );
}
