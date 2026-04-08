import { CURRENT_VERSION, GITHUB_REPO_URL } from './site';

export type InstallTargetId = 'windows-x64' | 'linux-x64' | 'macos-arm64' | 'macos-x64';

export type InstallOption = {
    id: InstallTargetId;
    label: string;
    family: 'windows' | 'linux' | 'macos';
    architecture: 'x64' | 'arm64';
    assetName: string;
    archiveType: '.zip' | '.tar.gz';
    summary: string;
    accentClass: string;
};

export type InstallReleaseSummary = {
    versionLabel: string;
    publishedAt: string | null;
    releaseUrl: string;
    checksumsUrl: string | null;
    availableAssets: string[];
};

type GithubReleaseAsset = {
    name?: unknown;
};

type GithubReleaseResponse = {
    tag_name?: unknown;
    html_url?: unknown;
    published_at?: unknown;
    assets?: unknown;
};

type NavigatorLike = {
    userAgent?: string;
    platform?: string;
    userAgentData?: {
        platform?: string;
        architecture?: string;
    };
};

const GITHUB_REPOSITORY_PATH = extractRepositoryPath(GITHUB_REPO_URL);

export const INSTALL_OPTIONS: InstallOption[] = [
    {
        id: 'windows-x64',
        label: 'Windows x64',
        family: 'windows',
        architecture: 'x64',
        assetName: 'arden-windows-x64-portable.zip',
        archiveType: '.zip',
        summary: 'Portable bundle with Arden, LLVM, and the Windows launcher script.',
        accentClass: 'from-[#7a432c] to-[#b85c38]',
    },
    {
        id: 'linux-x64',
        label: 'Linux x64',
        family: 'linux',
        architecture: 'x64',
        assetName: 'arden-linux-x64-portable.tar.gz',
        archiveType: '.tar.gz',
        summary: 'Portable archive with Arden, LLVM tooling, and linker helpers for Linux.',
        accentClass: 'from-[#4d5c40] to-[#7a8b63]',
    },
    {
        id: 'macos-arm64',
        label: 'macOS Apple Silicon',
        family: 'macos',
        architecture: 'arm64',
        assetName: 'arden-macos-arm64-portable.tar.gz',
        archiveType: '.tar.gz',
        summary: 'Best choice for M-series Macs with the bundled LLVM and lld toolchain.',
        accentClass: 'from-[#4b4b63] to-[#7f7fb0]',
    },
    {
        id: 'macos-x64',
        label: 'macOS Intel',
        family: 'macos',
        architecture: 'x64',
        assetName: 'arden-macos-x64-portable.tar.gz',
        archiveType: '.tar.gz',
        summary: 'Portable build for Intel Macs with the expected LLVM and lld layout.',
        accentClass: 'from-[#5d4f6d] to-[#8d729f]',
    },
];

export const INSTALL_DOCS_PATH = '/docs/getting_started/installation';
export const INSTALL_PAGE_PATH = '/install';
export const LATEST_RELEASE_API_URL = `https://api.github.com/repos/${GITHUB_REPOSITORY_PATH}/releases/latest`;

function extractRepositoryPath(repositoryUrl: string): string {
    const parsedUrl = new URL(repositoryUrl);
    return parsedUrl.pathname.replace(/^\/+/, '').replace(/\/+$/, '');
}

function isGithubReleaseResponse(value: unknown): value is GithubReleaseResponse {
    return Boolean(value) && typeof value === 'object';
}

function readString(value: unknown): string | null {
    return typeof value === 'string' && value.length > 0 ? value : null;
}

function readAssetNames(value: unknown): string[] {
    if (!Array.isArray(value)) {
        return [];
    }

    return value
        .map((asset) => {
            if (!asset || typeof asset !== 'object') {
                return null;
            }

            return readString((asset as GithubReleaseAsset).name);
        })
        .filter((name): name is string => name !== null);
}

function normalizeVersionLabel(tagName: string | null): string {
    if (!tagName) {
        return CURRENT_VERSION;
    }

    return tagName.startsWith('v') ? tagName : `v${tagName}`;
}

export function getLatestDownloadUrl(option: InstallOption): string {
    return `${GITHUB_REPO_URL}/releases/latest/download/${option.assetName}`;
}

export function getLatestChecksumsDownloadUrl(): string {
    return `${GITHUB_REPO_URL}/releases/latest/download/SHA256SUMS.txt`;
}

export function getFallbackReleaseUrl(): string {
    return `${GITHUB_REPO_URL}/releases/tag/${CURRENT_VERSION}`;
}

export function detectPreferredInstallTarget(navigatorLike: NavigatorLike): InstallTargetId | null {
    const platform = `${navigatorLike.userAgentData?.platform ?? navigatorLike.platform ?? ''}`.toLowerCase();
    const architecture = `${navigatorLike.userAgentData?.architecture ?? ''}`.toLowerCase();
    const userAgent = `${navigatorLike.userAgent ?? ''}`.toLowerCase();

    if (platform.includes('win')) {
        return 'windows-x64';
    }

    if (platform.includes('linux') || userAgent.includes('linux')) {
        return 'linux-x64';
    }

    if (platform.includes('mac') || userAgent.includes('mac os')) {
        if (architecture.includes('arm') || userAgent.includes('arm64') || userAgent.includes('aarch64')) {
            return 'macos-arm64';
        }
        return 'macos-x64';
    }

    return null;
}

export function getRecommendedInstallOption(targetId: InstallTargetId | null): InstallOption {
    if (!targetId) {
        return INSTALL_OPTIONS[0];
    }

    return INSTALL_OPTIONS.find((option) => option.id === targetId) ?? INSTALL_OPTIONS[0];
}

export async function fetchLatestReleaseSummary(signal?: AbortSignal): Promise<InstallReleaseSummary | null> {
    const response = await fetch(LATEST_RELEASE_API_URL, {
        headers: {
            Accept: 'application/vnd.github+json',
        },
        signal,
    });

    if (!response.ok) {
        return null;
    }

    const json: unknown = await response.json();
    if (!isGithubReleaseResponse(json)) {
        return null;
    }

    const releaseUrl = readString(json.html_url) ?? getFallbackReleaseUrl();
    const availableAssets = readAssetNames(json.assets);

    return {
        versionLabel: normalizeVersionLabel(readString(json.tag_name)),
        publishedAt: readString(json.published_at),
        releaseUrl,
        checksumsUrl: availableAssets.includes('SHA256SUMS.txt') ? getLatestChecksumsDownloadUrl() : null,
        availableAssets,
    };
}
