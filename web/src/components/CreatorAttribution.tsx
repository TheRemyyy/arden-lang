import { SITE_CREATOR_NAME, SITE_CREATOR_URL } from '../lib/site';

type CreatorAttributionProps = {
    className?: string;
};

export function CreatorAttribution({ className = '' }: CreatorAttributionProps) {
    return (
        <div
            className={`rounded-[1.5rem] border border-white/10 bg-white/[0.04] px-5 py-4 text-sm leading-7 text-white/68 ${className}`.trim()}
        >
            Built and maintained by{' '}
            <a
                href={SITE_CREATOR_URL}
                target="_blank"
                rel="me author noopener noreferrer"
                className="font-semibold text-white transition-colors hover:text-[var(--accent-soft)]"
            >
                {SITE_CREATOR_NAME}
            </a>
            . Arden is open source under Apache 2.0 and published at{' '}
            <a
                href={SITE_CREATOR_URL}
                target="_blank"
                rel="me author noopener noreferrer"
                className="font-semibold text-white transition-colors hover:text-[var(--accent-soft)]"
            >
                theremyyy.dev
            </a>
            .
        </div>
    );
}
