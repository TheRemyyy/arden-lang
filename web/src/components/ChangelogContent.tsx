import { motion } from 'framer-motion';

export function ChangelogContent({ html }: { html: string }) {
    return (
        <div className="flex-1 min-h-screen w-full max-w-4xl mx-auto overflow-x-hidden bg-[#09090b] px-6 pb-20 pt-24">
            <div className="mb-16 text-center">
                <h1 className="mb-6 text-5xl font-bold tracking-tight text-white">Changelog</h1>
                <p className="mx-auto max-w-2xl text-lg font-medium text-gray-400">
                    Tracking the latest improvements to Arden.
                </p>
            </div>

            <motion.div
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.3 }}
                className="rounded-2xl border border-[#1f1f23] bg-[#0c0c0e] p-8 shadow-xl md:p-12"
            >
                <article
                    className="prose prose-invert prose-zinc max-w-none break-words
                        prose-h1:text-3xl prose-h1:font-bold prose-h1:text-white
                        prose-h2:border-b prose-h2:border-[#27272a] prose-h2:pb-2 prose-h2:text-2xl prose-h2:font-semibold prose-h2:text-gray-100
                        prose-p:text-gray-300
                        prose-li:text-gray-300
                        prose-code:rounded prose-code:bg-[#18181b] prose-code:px-1 prose-code:text-gray-200
                        prose-pre:border prose-pre:border-[#27272a] prose-pre:bg-[#0c0c0e]"
                    dangerouslySetInnerHTML={{ __html: html }}
                />
            </motion.div>
        </div>
    );
}
