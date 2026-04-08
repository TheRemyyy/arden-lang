import { describe, expect, it } from 'vitest';
import {
    detectPreferredInstallTarget,
    getLatestChecksumsDownloadUrl,
    getLatestDownloadUrl,
    INSTALL_OPTIONS,
    LATEST_RELEASE_API_URL,
} from './install';

describe('install helpers', () => {
    it('builds latest-release download URLs for every portable asset', () => {
        for (const option of INSTALL_OPTIONS) {
            expect(getLatestDownloadUrl(option)).toContain('/releases/latest/download/');
            expect(getLatestDownloadUrl(option)).toContain(option.assetName);
        }
        expect(getLatestChecksumsDownloadUrl()).toBe(
            'https://github.com/TheRemyyy/apex-compiler/releases/latest/download/SHA256SUMS.txt',
        );
    });

    it('detects Windows and Linux targets', () => {
        expect(detectPreferredInstallTarget({ platform: 'Win32' })).toBe('windows-x64');
        expect(detectPreferredInstallTarget({ userAgent: 'Mozilla/5.0 (X11; Linux x86_64)' })).toBe('linux-x64');
    });

    it('detects macOS architecture when browser hints are available', () => {
        expect(
            detectPreferredInstallTarget({
                platform: 'MacIntel',
                userAgentData: { platform: 'macOS', architecture: 'arm' },
            }),
        ).toBe('macos-arm64');
        expect(detectPreferredInstallTarget({ platform: 'MacIntel' })).toBe('macos-x64');
    });

    it('points metadata fetches at the GitHub latest release API', () => {
        expect(LATEST_RELEASE_API_URL).toBe('https://api.github.com/repos/TheRemyyy/apex-compiler/releases/latest');
    });
});
