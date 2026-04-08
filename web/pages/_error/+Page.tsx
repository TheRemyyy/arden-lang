import { usePageContext } from 'vike-react/usePageContext';

type ErrorWithMessage = {
    message?: string;
};

function getErrorMessage(value: unknown): string {
    if (typeof value === 'string') {
        return value;
    }

    if (value && typeof value === 'object' && 'message' in value) {
        return (value as ErrorWithMessage).message ?? 'Unexpected error';
    }

    return 'Unexpected error';
}

export default function Page() {
    const pageContext = usePageContext();
    const statusCode = pageContext.abortStatusCode ?? (pageContext.is404 ? 404 : 500);
    const title = pageContext.is404 ? 'Page not found' : 'Something broke while rendering Arden';
    const message = pageContext.is404
        ? 'The requested page does not exist or is no longer available.'
        : getErrorMessage(pageContext.abortReason ?? pageContext.errorWhileRendering);

    return (
        <div className="flex min-h-screen w-full items-center justify-center px-6 pb-10 pt-24 md:px-10 md:pb-14 md:pt-28">
            <section className="paper-panel w-full max-w-4xl rounded-[2rem] p-8 md:p-12">
                <p className="text-xs font-semibold uppercase tracking-[0.24em] text-[var(--accent)]">
                    Error {statusCode}
                </p>
                <h1 className="mt-4 max-w-3xl font-display text-4xl font-bold tracking-[-0.04em] text-[var(--text)] md:text-6xl">
                    {title}
                </h1>
                <p className="mt-5 max-w-2xl text-base leading-8 text-[var(--text-muted)]">
                    {message}
                </p>
                <div className="mt-8 flex flex-wrap gap-3">
                    <a
                        href="/"
                        className="inline-flex h-12 items-center rounded-full bg-[var(--bg-strong)] px-6 text-sm font-semibold text-white transition-transform hover:-translate-y-0.5"
                    >
                        Back to home
                    </a>
                    <a
                        href="/docs/overview"
                        className="inline-flex h-12 items-center rounded-full border border-[rgba(57,52,46,0.16)] bg-white/70 px-6 text-sm font-semibold text-[var(--text)] transition-colors hover:border-[var(--accent)] hover:text-[var(--accent)]"
                    >
                        Open documentation
                    </a>
                </div>
            </section>
        </div>
    );
}
