import type { ReactNode } from 'react';
import { usePageContext } from 'vike-react/usePageContext';
import { Footer } from '../src/components/Footer';
import { Header } from '../src/components/Header';
import '../src/index.css';

export default function Layout({ children }: { children: ReactNode }) {
    const pageContext = usePageContext();
    const isDocsPage = pageContext.urlPathname.startsWith('/docs');

    return (
        <div className="flex min-h-screen flex-col bg-[#0a0a0a]">
            <Header />
            <main className="flex-grow">{children}</main>
            {!isDocsPage && <Footer />}
        </div>
    );
}
