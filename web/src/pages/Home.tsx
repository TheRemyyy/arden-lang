import { Link } from 'react-router-dom';
import { ArrowRight, Blocks, Shield, Terminal } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import { motion } from 'framer-motion';

const principles = [
    'Memory safety with less ceremony',
    'Native performance through LLVM',
    'Compiler feedback built for humans',
];

const features = [
    {
        icon: Shield,
        title: 'Ownership without the grind',
        desc: 'Ownership inference catches common pitfalls while keeping code readable in everyday workflows.',
    },
    {
        icon: Blocks,
        title: 'Abstractions that stay fast',
        desc: 'Generics compile into concrete machine code, so ergonomics do not add runtime cost.',
    },
    {
        icon: Terminal,
        title: 'Debuggable compile errors',
        desc: 'Error messages are specific and actionable, helping teams move from failure to fix faster.',
    },
];

export function Home() {
    return (
        <div className="min-h-screen overflow-x-hidden bg-[#0a0a0a] text-zinc-100">
            <section className="mx-auto grid w-full max-w-6xl gap-14 overflow-hidden px-6 pb-24 pt-36 lg:grid-cols-[1.1fr_0.9fr] lg:items-center">
                <motion.div initial={{ opacity: 0, y: 10 }} animate={{ opacity: 1, y: 0 }} className="min-w-0 space-y-8">
                    <div className="inline-flex items-center rounded-full border border-zinc-700 bg-zinc-900 px-3 py-1 text-xs font-medium text-zinc-300">
                        Apex v1.1.4
                    </div>
                    <h1 className="max-w-2xl text-4xl font-semibold leading-tight text-white md:text-6xl break-words">
                        Build systems software with speed, safety, and less friction.
                    </h1>
                    <p className="max-w-2xl text-lg leading-relaxed text-zinc-300 break-words">
                        Apex is a modern systems language built on LLVM. It gives teams low-level control and practical tooling without turning everyday development into a fight.
                    </p>
                    <div className="flex flex-wrap gap-3 pt-2">
                        <Link to="/docs/overview" className="inline-flex h-11 items-center gap-2 rounded-lg bg-white px-5 text-sm font-semibold text-black transition hover:bg-zinc-200">
                            Get started
                            <ArrowRight size={16} />
                        </Link>
                        <a
                            href="https://github.com/TheRemyyy/apex-compiler"
                            target="_blank"
                            rel="noreferrer"
                            className="inline-flex h-11 items-center rounded-lg border border-zinc-700 bg-zinc-900 px-5 text-sm font-medium text-zinc-200 transition hover:border-zinc-500 hover:bg-zinc-800"
                        >
                            View on GitHub
                        </a>
                    </div>
                </motion.div>

                <motion.div initial={{ opacity: 0, y: 14 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: 0.1 }} className="min-w-0">
                    <div className="rounded-2xl border border-zinc-700 bg-[#111111] p-6">
                        <div className="mb-4 flex items-center justify-between">
                            <p className="text-sm font-medium text-zinc-200">Range Iterator</p>
                            <span className="rounded-md bg-zinc-800 px-2 py-1 text-xs font-medium text-zinc-300">v1.3.2</span>
                        </div>
                        <div className="rounded-xl border border-zinc-700 bg-[#0d0d0d] p-5">
                            <pre className="overflow-x-auto whitespace-pre-wrap break-words text-sm leading-7 text-zinc-300">
                                <code>
                                    <div>import std.io.*;</div>
                                    <div></div>
                                    <div>function main(): None {'{'}</div>
                                    <div className="pl-4">r: Range&lt;Integer&gt; = range(0, 10, 2);</div>
                                    <div className="pl-4">while (r.has_next()) {'{'}</div>
                                    <div className="pl-8">println(to_string(r.next()));</div>
                                    <div className="pl-4">{'}'}</div>
                                    <div className="pl-4">return None;</div>
                                    <div>{'}'}</div>
                                </code>
                            </pre>
                        </div>
                    </div>
                </motion.div>
            </section>

            <section className="border-y border-zinc-800 bg-[#101010] py-20">
                <div className="mx-auto grid w-full max-w-6xl gap-10 px-6 lg:grid-cols-[1.2fr_0.8fr]">
                    <div className="max-w-2xl">
                        <h2 className="text-3xl font-semibold text-white md:text-4xl">Pragmatic by design</h2>
                        <p className="mt-4 text-base leading-relaxed text-zinc-300">
                            Apex focuses on the boring hard parts: predictable behavior, native output, and developer velocity. No inflated visual noise, no framework theater.
                        </p>
                    </div>
                    <ul className="space-y-3">
                        {principles.map((item) => (
                            <li key={item} className="rounded-lg border border-zinc-700 bg-[#141414] px-4 py-3 text-sm text-zinc-200">
                                {item}
                            </li>
                        ))}
                    </ul>
                </div>
            </section>

            <section className="mx-auto w-full max-w-6xl px-6 py-20">
                <div className="mb-10 max-w-2xl">
                    <h2 className="text-3xl font-semibold text-white md:text-4xl">Core capabilities</h2>
                    <p className="mt-4 text-base leading-relaxed text-zinc-300">
                        A focused feature set for teams building performance-sensitive software.
                    </p>
                </div>
                <div className="divide-y divide-zinc-800 border-y border-zinc-800">
                    {features.map((feature) => (
                        <FeatureRow key={feature.title} icon={feature.icon} title={feature.title} desc={feature.desc} />
                    ))}
                </div>
            </section>
        </div>
    );
}

function FeatureRow({ icon: Icon, title, desc }: { icon: LucideIcon; title: string; desc: string }) {
    return (
        <article className="grid gap-4 py-8 md:grid-cols-[44px_1fr] md:items-start">
            <div className="inline-flex h-11 w-11 items-center justify-center rounded-lg border border-zinc-700 bg-[#161616]">
                <Icon size={18} className="text-zinc-200" />
            </div>
            <div>
                <h3 className="text-xl font-semibold text-white">{title}</h3>
                <p className="mt-2 max-w-3xl text-sm leading-relaxed text-zinc-300">{desc}</p>
            </div>
        </article>
    );
}




