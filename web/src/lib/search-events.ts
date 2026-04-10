export const SITE_SEARCH_OPEN_EVENT = 'arden:search-open';

export function openSiteSearch() {
    if (typeof window === 'undefined') return;
    window.dispatchEvent(new Event(SITE_SEARCH_OPEN_EVENT));
}
