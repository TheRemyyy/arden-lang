import { FLATTENED_DOCS } from '../../src/lib/docs';

export default function onBeforePrerenderStart() {
    return ['/docs', ...FLATTENED_DOCS.map((doc) => doc.path)];
}
