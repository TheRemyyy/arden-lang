type LegalPageProps = {
    eyebrow: string;
    title: string;
    intro: string;
    sections: Array<{
        title: string;
        body: string[];
    }>;
};

export function LegalPage({ eyebrow, title, intro, sections }: LegalPageProps) {
    return (
        <div className="min-h-screen bg-[#0f0d0b] px-6 pb-24 pt-28 text-[#f3ece3]">
            <div className="mx-auto max-w-4xl">
                <p className="text-xs font-semibold uppercase tracking-[0.22em] text-[var(--accent-soft)]">
                    {eyebrow}
                </p>
                <h1 className="mt-4 font-display text-4xl font-bold tracking-[-0.04em] text-white md:text-5xl">
                    {title}
                </h1>
                <p className="mt-5 max-w-3xl text-base leading-8 text-white/68">
                    {intro}
                </p>

                <div className="mt-10 space-y-5">
                    {sections.map((section) => (
                        <section
                            key={section.title}
                            className="rounded-[1.6rem] border border-white/10 bg-white/[0.04] p-6"
                        >
                            <h2 className="text-2xl font-semibold tracking-[-0.03em] text-white">
                                {section.title}
                            </h2>
                            <div className="mt-4 space-y-4">
                                {section.body.map((paragraph) => (
                                    <p key={paragraph} className="text-sm leading-7 text-white/68">
                                        {paragraph}
                                    </p>
                                ))}
                            </div>
                        </section>
                    ))}
                </div>
            </div>
        </div>
    );
}

