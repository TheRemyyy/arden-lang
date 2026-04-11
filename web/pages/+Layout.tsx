import type { ReactNode } from 'react';
import { usePageContext } from 'vike-react/usePageContext';
import { DeferredTelemetry } from '../src/components/DeferredTelemetry';
import { Footer } from '../src/components/Footer';
import { Header } from '../src/components/Header';
import { SiteSearch } from '../src/components/SiteSearch';
import '../src/index.css';

export default function Layout({ children }: { children: ReactNode }) {
    const pageContext = usePageContext();
    const isDocsPage = pageContext.urlPathname.startsWith('/docs');

    return (
        <div className="flex min-h-screen flex-col bg-[var(--bg)] text-[var(--text)]">
            <Header />
            <SiteSearch />
            <main className="flex-grow">{children}</main>
            {!isDocsPage && <Footer />}
            <DeferredTelemetry />
        </div>
    );
}
