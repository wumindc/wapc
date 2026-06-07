/**
 * Deep-link runtime adapter for wapc://import entry points.
 * @author codex
 */

export interface DeepLinkRuntime {
  getCurrent: () => Promise<string[] | null>
  onOpenUrl: (handler: (urls: string[]) => void) => Promise<() => void>
}

export function isWapcImportDeepLink(url: string): boolean {
  return url.trim().startsWith('wapc://import?')
}

export async function attachWapcDeepLinkHandler(
  runtime: DeepLinkRuntime,
  handler: (url: string) => void,
): Promise<() => void> {
  const dispatch = (urls: string[] | null) => {
    for (const url of urls ?? []) {
      const trimmed = url.trim()
      if (isWapcImportDeepLink(trimmed)) {
        handler(trimmed)
      }
    }
  }

  dispatch(await runtime.getCurrent())
  return runtime.onOpenUrl(dispatch)
}
