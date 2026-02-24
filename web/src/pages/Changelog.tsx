import { useEffect, useState } from 'react';
import { marked } from 'marked';
import { motion } from 'framer-motion';

export function Changelog() {
    const [html, setHtml] = useState('');

    useEffect(() => {
        fetch('/CHANGELOG.md')
            .then(res => res.text())
            .then(async text => {
                const parsed = await marked.parse(text);
                setHtml(parsed);
            })
            .catch(() => setHtml('<h1>Changelog not found</h1>'));
    }, []);

    return (
        <div className="flex-1 pt-24 pb-20 px-6 max-w-4xl mx-auto w-full bg-[#09090b] min-h-screen overflow-x-hidden">
            <div className="mb-16 text-center">
                <h1 className="text-5xl font-bold text-white mb-6 tracking-tight">Changelog</h1>
                <p className="text-lg text-gray-400 font-medium max-w-2xl mx-auto">Tracking the latest improvements to Apex.</p>
            </div>

            <motion.div
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.3 }}
                className="bg-[#0c0c0e] border border-[#1f1f23] rounded-2xl p-8 md:p-12 shadow-xl"
            >
                <article className="prose prose-invert prose-zinc max-w-none break-words
                        prose-h1:text-3xl prose-h1:font-bold prose-h1:text-white
                        prose-h2:text-2xl prose-h2:font-semibold prose-h2:text-gray-100 prose-h2:border-b prose-h2:border-[#27272a] prose-h2:pb-2
                        prose-p:text-gray-300
                        prose-li:text-gray-300
                        prose-code:text-gray-200 prose-code:bg-[#18181b] prose-code:px-1 prose-code:rounded
                        prose-pre:bg-[#0c0c0e] prose-pre:border prose-pre:border-[#27272a]"
                    dangerouslySetInnerHTML={{ __html: html }}
                />
            </motion.div>
        </div>
    );
}
