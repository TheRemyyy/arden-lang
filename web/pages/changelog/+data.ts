import { loadChangelogPage } from '../../src/lib/content.server';

export async function data() {
    return loadChangelogPage();
}
