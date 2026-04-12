import { Routes, Route, useLocation } from 'react-router-dom';
import { Suspense, lazy } from 'react';
import { Header } from './components/Header';
import { Footer } from './components/Footer';
import { DeferredTelemetry } from './components/DeferredTelemetry';
import { Home } from './pages/Home';

const Docs = lazy(() => import('./pages/Docs').then(module => ({ default: module.Docs })));
const Changelog = lazy(() => import('./pages/Changelog').then(module => ({ default: module.Changelog })));

export default function App() {
    const location = useLocation();
    const isDocsPage = location.pathname.startsWith('/docs');

    return (
        <div className="min-h-screen flex flex-col text-[var(--text)]">
            <Header />
            <main className="flex-grow">
                <Suspense fallback={<div className="flex-1 bg-[var(--bg)]" />}>
                  <Routes>
                      <Route path="/" element={<Home />} />
                      <Route path="/docs/*" element={<Docs />} />
                      <Route path="/changelog" element={<Changelog />} />
                  </Routes>
                </Suspense>
            </main>
            {!isDocsPage && <Footer />}
            <DeferredTelemetry />
        </div>
    );
}
