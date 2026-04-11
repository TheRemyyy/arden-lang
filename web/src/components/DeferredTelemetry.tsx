import { lazy, Suspense, useEffect, useState } from 'react';

const Analytics = lazy(async () => {
    const module = await import('@vercel/analytics/react');
    return { default: module.Analytics };
});

const SpeedInsights = lazy(async () => {
    const module = await import('@vercel/speed-insights/react');
    return { default: module.SpeedInsights };
});

function scheduleIdleCallback(callback: () => void): () => void {
    if (typeof window === 'undefined') {
        return () => {};
    }

    if ('requestIdleCallback' in window) {
        const handle = window.requestIdleCallback(callback, { timeout: 2000 });
        return () => window.cancelIdleCallback(handle);
    }

    const timeout = globalThis.setTimeout(callback, 1200);
    return () => globalThis.clearTimeout(timeout);
}

export function DeferredTelemetry() {
    const [isEnabled, setIsEnabled] = useState(false);

    useEffect(() => {
        const isProductionHost =
            typeof window !== 'undefined' &&
            ['arden-lang.dev', 'www.arden-lang.dev'].includes(window.location.hostname);

        if (!isProductionHost) {
            return () => {};
        }

        const cancel = scheduleIdleCallback(() => setIsEnabled(true));
        return cancel;
    }, []);

    if (!isEnabled) {
        return null;
    }

    return (
        <Suspense fallback={null}>
            <Analytics />
            <SpeedInsights />
        </Suspense>
    );
}
