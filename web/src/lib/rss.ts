export function slugifyRss(value: string): string {
    return (
        value
            .toLowerCase()
            .replace(/[^\p{L}\p{N}]+/gu, '-')
            .replace(/^-+|-+$/g, '') || 'release'
    );
}
