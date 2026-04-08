import { renderMarkdown } from './markdown';

export type ChangelogCategory = {
    id: string;
    title: string;
    plainTitle: string;
    kind: string;
    html: string;
    itemCount: number;
};

export type ChangelogRelease = {
    id: string;
    version: string;
    label: string;
    subtitle: string | null;
    displayTitle: string;
    date: string | null;
    summaryHtml: string | null;
    categories: ChangelogCategory[];
};

function slugify(value: string): string {
    return value
        .toLowerCase()
        .replace(/[^\p{L}\p{N}]+/gu, '-')
        .replace(/^-+|-+$/g, '') || 'section';
}

function stripEmoji(value: string): string {
    return value.replace(/^[^\p{L}\p{N}]+/gu, '').trim();
}

function countTopLevelItems(markdown: string): number {
    return markdown
        .split('\n')
        .filter((line) => /^- /.test(line.trim()))
        .length;
}

function splitSections(markdown: string, headingPattern: RegExp) {
    const matches = Array.from(markdown.matchAll(headingPattern));
    return matches.map((match, index) => {
        const title = match[1].trim();
        const start = match.index ?? 0;
        const bodyStart = start + match[0].length;
        const end = matches[index + 1]?.index ?? markdown.length;
        return {
            title,
            body: markdown.slice(bodyStart, end).trim(),
        };
    });
}

function parseReleaseHeading(rawHeading: string) {
    const match = rawHeading.match(/^\[(.+?)\](?: - (.*?))?(?: - (\d{4}-\d{2}-\d{2}))?$/);
    if (!match) {
        const plain = rawHeading.trim();
        return {
            version: plain,
            label: plain,
            subtitle: null,
            date: null,
        };
    }

    const [, version, subtitle, date] = match;
    return {
        version,
        label: version === 'Unreleased' ? 'Unreleased' : `v${version}`,
        subtitle: subtitle?.trim() || null,
        date: date ?? null,
    };
}

export async function parseChangelogMarkdown(markdown: string): Promise<ChangelogRelease[]> {
    const releases = splitSections(markdown, /^##\s+(.+)$/gm);

    return Promise.all(
        releases.map(async (release) => {
            const { version, label, subtitle, date } = parseReleaseHeading(release.title);
            const categories = splitSections(release.body, /^###\s+(.+)$/gm);
            const summaryMarkdown = categories.length > 0
                ? release.body.slice(0, release.body.indexOf(`### ${categories[0].title}`)).trim()
                : release.body.trim();
            const displayTitle = subtitle ?? (version === 'Unreleased' ? 'Current development branch' : `Release ${label}`);

            return {
                id: slugify(version),
                version,
                label,
                subtitle,
                displayTitle,
                date,
                summaryHtml: summaryMarkdown ? await renderMarkdown(summaryMarkdown) : null,
                categories: await Promise.all(
                    categories.map(async (category) => {
                        const normalizedTitle = stripEmoji(category.title);
                        return {
                            id: `${slugify(version)}-${slugify(normalizedTitle)}`,
                            title: category.title,
                            plainTitle: normalizedTitle,
                            kind: normalizedTitle.toLowerCase(),
                            html: await renderMarkdown(category.body),
                            itemCount: countTopLevelItems(category.body),
                        };
                    }),
                ),
            };
        }),
    );
}
