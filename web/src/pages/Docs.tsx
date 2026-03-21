import { useState, useEffect } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { renderMarkdown, rewriteInternalDocLinks } from '../lib/markdown';

// Navigation structure - URL paths (without .md)
const NAV_ITEMS = [
    { title: 'Overview', path: '/docs/overview' },
    {
        title: 'Getting Started', items: [
            { title: 'Installation', path: '/docs/getting_started/installation' },
            { title: 'Quick Start', path: '/docs/getting_started/quick_start' },
            { title: 'Editor Setup', path: '/docs/getting_started/editor_setup' },
        ]
    },
    {
        title: 'Basics', items: [
            { title: 'Syntax', path: '/docs/basics/syntax' },
            { title: 'Variables', path: '/docs/basics/variables' },
            { title: 'Types', path: '/docs/basics/types' },
            { title: 'Control Flow', path: '/docs/basics/control_flow' },
        ]
    },
    {
        title: 'Features', items: [
            { title: 'Functions', path: '/docs/features/functions' },
            { title: 'Classes', path: '/docs/features/classes' },
            { title: 'Interfaces', path: '/docs/features/interfaces' },
            { title: 'Enums', path: '/docs/features/enums' },
            { title: 'Ranges', path: '/docs/features/ranges' },
            { title: 'Modules', path: '/docs/features/modules' },
            { title: 'Projects', path: '/docs/features/projects' },
        ]
    },
    {
        title: 'Standard Library', items: [
            { title: 'Overview', path: '/docs/stdlib/overview' },
            { title: 'Math', path: '/docs/stdlib/math' },
            { title: 'Str', path: '/docs/stdlib/string' },
            { title: 'Time', path: '/docs/stdlib/time' },
            { title: 'Args', path: '/docs/stdlib/args' },
            { title: 'Collections', path: '/docs/stdlib/collections' },
            { title: 'I/O', path: '/docs/stdlib/io' },
            { title: 'System', path: '/docs/stdlib/system' },
        ]
    },
    {
        title: 'Advanced', items: [
            { title: 'Ownership', path: '/docs/advanced/ownership' },
            { title: 'Generics', path: '/docs/advanced/generics' },
            { title: 'Async/Await', path: '/docs/advanced/async' },
            { title: 'Error Handling', path: '/docs/advanced/error_handling' },
            { title: 'Memory Management', path: '/docs/advanced/memory_management' },        
        ]
    },
    {
        title: 'Compiler', items: [
            { title: 'CLI', path: '/docs/compiler/cli' },
            { title: 'Architecture', path: '/docs/compiler/architecture' },
        ]
    }
];

// Dynamic Table Of Contents Component
function TableOfContents({ html }: { html: string }) {
    const [headings, setHeadings] = useState<{ id: string, text: string }[]>([]);

    useEffect(() => {
        const tempDiv = document.createElement('div');
        tempDiv.innerHTML = html;
        const headers = Array.from(tempDiv.querySelectorAll('h2[id]'));

        const extracted = headers.map(h => {
            const text = h.textContent || '';
            const id = h.getAttribute('id') || '';
            return { id, text };
        });
        setHeadings(extracted);
    }, [html]);

    if (!html) return null;

    return (
        <div className="fixed w-64 right-0 top-0 h-full p-8 pt-24 border-l border-[#1f1f23] hidden xl:block overflow-y-auto">
            <h5 className="text-xs font-bold text-gray-500 uppercase tracking-widest mb-4">On This Page</h5>
            <ul className="space-y-3">
                {headings.map((h, i) => (
                    <li key={i}>
                        <a href={`#${h.id}`} className="text-[13px] text-gray-400 hover:text-[#a5b4fc] transition-colors block leading-snug outline-none focus:text-white">
                            {h.text}
                        </a>
                    </li>
                ))}
            </ul>
        </div>
    );
}

// Helper to flatten ALL clickable items for next/prev logic
const FLATTENED_DOCS = NAV_ITEMS.reduce((acc: {title: string, path: string}[], section) => {
    if (section.path) acc.push({ title: section.title, path: section.path });
    if (section.items) section.items.forEach(item => acc.push(item));
    return acc;
}, []);

export function Docs() {
    const location = useLocation();
    const navigate = useNavigate();
    const [content, setContent] = useState('');
    const [loading, setLoading] = useState(true);
    const [isSidebarOpen, setIsSidebarOpen] = useState(false);

    // Normalize path (without .md)
    const normalizedPath = normalizeDocsPath(location.pathname);
    
    // Fetch path (with .md extension)
    const fetchPath = normalizedPath + '.md';

    // Calculate Next/Prev
    const currentIndex = FLATTENED_DOCS.findIndex(item => item.path === normalizedPath);        
    const prevDoc = currentIndex > 0 ? FLATTENED_DOCS[currentIndex - 1] : null;
    const nextDoc = currentIndex !== -1 && currentIndex < FLATTENED_DOCS.length - 1 ? FLATTENED_DOCS[currentIndex + 1] : null;

    useEffect(() => {
        setIsSidebarOpen(false);
        setLoading(true);
        setContent('');
        const controller = new AbortController();

        fetch(fetchPath, { signal: controller.signal })
            .then(res => {
                if (!res.ok) throw new Error('Not found');
                return res.text();
            })
            .then(async text => {
                const html = await renderMarkdown(text);
                const rewrittenHtml = rewriteInternalDocLinks(html, normalizedPath);
                setContent(rewrittenHtml);
                setLoading(false);
                window.scrollTo(0, 0);
            })
            .catch(err => {
                if (err instanceof DOMException && err.name === 'AbortError') {
                    return;
                }
                console.error(err);
                setContent('<h1>Document not found</h1><p>The requested page could not be found.</p>');
                setLoading(false);
            });
        return () => controller.abort();
    }, [fetchPath, normalizedPath]);

    const handleContentClick = (event: React.MouseEvent<HTMLElement>) => {
        const target = event.target as HTMLElement;
        const link = target.closest('a[data-router-link="true"]') as HTMLAnchorElement | null;
        if (!link) return;

        const href = link.getAttribute('href');
        if (!href) return;

        event.preventDefault();
        navigate(href);
    };

    return (
        <div className="flex flex-col lg:flex-row bg-[#09090b] text-gray-100 font-sans selection:bg-gray-700 selection:text-white pt-14 lg:pt-16 min-h-screen">

            {/* Mobile Sidebar Toggle Bar */}
            <div className="lg:hidden fixed top-14 left-0 right-0 h-10 bg-[#0c0c0e] border-b border-[#1f1f23] flex items-center px-4 z-20">
                <button
                    onClick={() => setIsSidebarOpen(!isSidebarOpen)}
                    className="text-sm font-medium text-white flex items-center gap-2"
                >
                    <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h16M4 18h16" /></svg>
                    Menu
                </button>
                <span className="ml-auto text-xs text-gray-500 font-mono">
                    {NAV_ITEMS.find(s => s.items?.some(i => i.path === normalizedPath) || s.path === normalizedPath)?.title || 'Documentation'}
                </span>
            </div>

            {/* Sidebar */}
            <nav className={`fixed lg:fixed w-72 left-0 top-16 h-[calc(100vh-4rem)] border-r border-[#1f1f23] bg-[#0c0c0e] flex flex-col z-30 transition-transform duration-300 ${isSidebarOpen ? 'translate-x-0' : '-translate-x-full lg:translate-x-0'}`}>
                <div className="flex-1 overflow-y-auto p-6 custom-scrollbar space-y-8 pb-20">   
                    {NAV_ITEMS.map((section, idx) => (
                        <section key={idx}>
                            {section.items ? (
                                <>
                                    <h3 className="text-[11px] font-bold text-gray-500 uppercase tracking-widest mb-3 pl-2">
                                        {section.title}
                                    </h3>
                                    <ul className="space-y-0.5">
                                        {section.items.map((item, itemIdx) => {
                                            const isActive = normalizedPath === item.path;      
                                            return (
                                                <li key={itemIdx}>
                                                    <button
                                                        onClick={() => navigate(item.path)}     
                                                        className={`w-full text-left px-3 py-1.5 rounded-md text-[14px] font-medium transition-colors duration-100 outline-none focus:outline-none focus:ring-0 border ${isActive
                                                            ? 'bg-[#18181b] text-white border-[#27272a]'
                                                            : 'text-gray-400 hover:text-gray-200 hover:bg-[#18181b]/50 border-transparent'
                                                            }`}
                                                    >
                                                        {item.title}
                                                    </button>
                                                </li>
                                            );
                                        })}
                                    </ul>
                                </>
                            ) : (
                                <button
                                    onClick={() => navigate(section.path!)}
                                    className={`w-full text-left px-3 py-1.5 rounded-md text-[14px] font-bold uppercase tracking-wider mb-2 transition-colors duration-100 outline-none focus:outline-none focus:ring-0 border ${normalizedPath === section.path ? 'bg-[#18181b] text-white border-[#27272a]' : 'text-gray-500 hover:text-gray-300 hover:bg-[#18181b]/50 border-transparent'    
                                        }`}
                                >
                                    {section.title}
                                </button>
                            )}
                        </section>
                    ))}
                </div>
            </nav>

            {/* Overlay for mobile sidebar */}
            {isSidebarOpen && (
                <div
                    className="lg:hidden fixed inset-0 z-20 bg-black/50 backdrop-blur-sm top-14"
                    onClick={() => setIsSidebarOpen(false)}
                />
            )}

            {/* Center Layout */}
            <div className="flex-1 lg:ml-72 xl:mr-64 w-full pt-10 lg:pt-0">
                <main className="max-w-4xl mx-auto px-6 md:px-12 py-10 lg:py-16 w-full min-h-[80vh]">
                    {loading ? (
                        <div className="animate-pulse space-y-8 pt-4">
                            <div className="h-10 bg-[#1f1f23] rounded w-1/2 mb-8"></div>        
                            <div className="space-y-4">
                                <div className="h-4 bg-[#1f1f23] rounded w-full"></div>
                                <div className="h-4 bg-[#1f1f23] rounded w-5/6"></div>
                                <div className="h-4 bg-[#1f1f23] rounded w-4/6"></div>
                            </div>
                        </div>
                    ) : (
                        <>
                            <article
                                className="prose prose-invert prose-zinc max-w-none
                        prose-headings:scroll-mt-24
                        prose-h1:text-3xl md:prose-h1:text-4xl prose-h1:font-bold prose-h1:tracking-tight prose-h1:mb-8 prose-h1:text-white
                        prose-h2:text-2xl prose-h2:font-semibold prose-h2:mt-12 prose-h2:mb-6 prose-h2:text-gray-100 prose-h2:border-b prose-h2:border-[#27272a] prose-h2:pb-2
                        prose-h3:text-xl prose-h3:font-semibold prose-h3:mt-8 prose-h3:mb-4 prose-h3:text-gray-200
                        prose-p:text-[15px] md:prose-p:text-[16px] prose-p:leading-7 prose-p:text-gray-300 prose-p:mb-6
                        prose-ul:my-6 prose-ul:list-disc prose-ul:pl-6 prose-li:text-gray-300 prose-li:mb-2
                        prose-strong:text-white prose-strong:font-semibold
                        prose-code:text-[13px] prose-code:bg-[#18181b] prose-code:px-1.5 prose-code:py-0.5 prose-code:rounded-md prose-code:border prose-code:border-[#27272a]/50
                        prose-pre:bg-[#0c0c0e] prose-pre:border prose-pre:border-[#27272a] prose-pre:rounded-lg prose-pre:shadow-sm"
                                onClick={handleContentClick}
                                dangerouslySetInnerHTML={{ __html: content }}
                            />

                            {/* Navigation Footer */}
                            <div className="mt-16 pt-8 border-t border-[#27272a] flex flex-col sm:flex-row justify-between gap-4">
                                <div>
                                    {prevDoc && (
                                        <button
                                            onClick={() => navigate(prevDoc.path)}
                                            className="group flex flex-col items-start gap-1 p-4 rounded-lg border border-[#27272a] hover:border-white/20 hover:bg-[#18181b] transition-all w-full sm:w-auto"
                                        >
                                            <span className="text-xs text-gray-500 font-medium uppercase tracking-wider group-hover:text-gray-400 text-left">Previous</span>
                                            <span className="text-gray-200 font-medium group-hover:text-white text-left">{prevDoc.title}</span>
                                        </button>
                                    )}
                                </div>
                                <div className="flex justify-end text-right">
                                    {nextDoc && (
                                        <button
                                            onClick={() => navigate(nextDoc.path)}
                                            className="group flex flex-col items-end gap-1 p-4 rounded-lg border border-[#27272a] hover:border-white/20 hover:bg-[#18181b] transition-all w-full sm:w-auto"
                                        >
                                            <span className="text-xs text-gray-500 font-medium uppercase tracking-wider group-hover:text-gray-400 text-right">Next</span>
                                            <span className="text-gray-200 font-medium group-hover:text-white text-right">{nextDoc.title}</span>
                                        </button>
                                    )}
                                </div>
                            </div>

                            <TableOfContents html={content} />
                        </>
                    )}
                </main>
            </div>

        </div>
    );
}

function normalizeDocsPath(pathname: string): string {
    if (pathname === '/docs' || pathname === '/docs/') {
        return '/docs/overview';
    }

    if (pathname.endsWith('/')) {
        return pathname.slice(0, -1);
    }

    return pathname;
}
