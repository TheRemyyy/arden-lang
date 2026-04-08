import { motion } from 'framer-motion';

export function ChangelogContent({ html }: { html: string }) {
    return (
        <div className="mx-auto flex-1 min-h-screen w-full max-w-5xl overflow-x-hidden bg-[#0f0d0b] px-6 pb-20 pt-24 text-[#f3ece3]">
            <div className="mb-14 text-center">
                <p className="text-xs font-semibold uppercase tracking-[0.22em] text-[var(--accent-soft)]">
                    Release history
                </p>
                <h1 className="mt-5 mb-5 font-display text-5xl font-bold tracking-[-0.04em] text-white">
                    Changelog
                </h1>
                <p className="mx-auto max-w-2xl text-lg leading-8 text-white/68">
                    Tracking the latest improvements to Arden.
                </p>
            </div>

            <motion.div
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.3 }}
                className="rounded-[2rem] border border-white/10 bg-[#161311] p-8 shadow-[0_24px_80px_rgba(0,0,0,0.28)] md:p-12"
            >
                <article
                    className="prose prose-invert prose-zinc max-w-none break-words
                        prose-headings:scroll-mt-24
                        prose-h1:font-display prose-h1:text-4xl prose-h1:font-bold prose-h1:tracking-[-0.04em] prose-h1:text-white
                        prose-h2:border-b prose-h2:border-white/10 prose-h2:pb-3 prose-h2:font-display prose-h2:text-3xl prose-h2:font-bold prose-h2:tracking-[-0.03em] prose-h2:text-white
                        prose-h3:font-display prose-h3:text-2xl prose-h3:font-semibold prose-h3:tracking-[-0.03em] prose-h3:text-[#f3ece3]
                        prose-p:text-[16px] prose-p:leading-8 prose-p:text-white/72
                        prose-li:text-white/72
                        prose-table:my-8 prose-table:w-full prose-table:border-collapse prose-table:text-left prose-thead:border-b prose-thead:border-white/12 prose-th:px-3 prose-th:pb-3 prose-th:text-xs prose-th:uppercase prose-th:tracking-[0.18em] prose-th:text-white/50 prose-td:border-b prose-td:border-white/8 prose-td:px-3 prose-td:py-3 prose-td:text-white/78
                        prose-strong:text-white
                        prose-a:text-[var(--accent-soft)] prose-a:no-underline hover:prose-a:text-white
                        prose-code:border-0 prose-code:bg-transparent prose-code:px-0 prose-code:py-0 prose-code:text-[13px] prose-code:text-[#f2d6c8] prose-code:before:content-none prose-code:after:content-none
                        prose-pre:rounded-[1.5rem] prose-pre:border prose-pre:border-white/10 prose-pre:bg-[#211e1a] prose-pre:text-[#f7efe5]
                        prose-blockquote:border-l-[var(--accent-soft)] prose-blockquote:text-white"
                    dangerouslySetInnerHTML={{ __html: html }}
                />
            </motion.div>
        </div>
    );
}
