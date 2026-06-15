import type { ElectronApplication } from '@playwright/test'

export type WindowPlacementTarget = 'primary' | 'secondary' | 'none'
export type WindowPlacementFallback = 'corner' | 'unchanged'

export type WindowPlacementRequest = {
  readonly target: WindowPlacementTarget
  readonly fallback: WindowPlacementFallback
  readonly strict: boolean
  readonly bounds?: {
    readonly width: number
    readonly height: number
  }
}

export type WindowPlacementResult = {
  readonly applied: boolean
  readonly requestedTarget: WindowPlacementTarget
  readonly actualTarget: Exclude<WindowPlacementTarget, 'none'> | 'unresolved'
  readonly fallbackUsed: boolean
  readonly targetHonored: boolean
  readonly message: string
  readonly warnings: readonly string[]
}

const targetEnvVar = 'DEKZER_E2E_WINDOW_TARGET'
const boundsEnvVar = 'DEKZER_E2E_WINDOW_BOUNDS'
const fallbackEnvVar = 'DEKZER_E2E_WINDOW_FALLBACK'
const strictEnvVar = 'DEKZER_E2E_WINDOW_STRICT'

export async function applyE2EWindowPlacement(
  app: ElectronApplication,
  log: (line: string) => void,
  env: NodeJS.ProcessEnv = process.env
): Promise<void> {
  const request = parseWindowPlacementRequest(env, log)

  if (request === undefined) {
    return
  }

  if (request.target === 'none') {
    log('[window-placement] disabled by DEKZER_E2E_WINDOW_TARGET=none.')
    return
  }

  log(
    '[window-placement] ' +
      `request target=${request.target}; fallback=${request.fallback}; ` +
      `strict=${String(request.strict)}; bounds=${formatRequestedBounds(request.bounds)}.`
  )

  try {
    const result = await app.evaluate(
      ({ BrowserWindow, screen }, placement): WindowPlacementResult => {
        const window = BrowserWindow.getAllWindows()[0]
        const warnings: string[] = []
        type DisplayLike = { readonly id: number }
        const displayTargetName = (
          display: DisplayLike,
          primaryDisplay: DisplayLike
        ): Exclude<WindowPlacementTarget, 'none'> =>
          display.id === primaryDisplay.id ? 'primary' : 'secondary'
        const waylandSession =
          process.platform === 'linux' &&
          (process.env.XDG_SESSION_TYPE?.toLowerCase() === 'wayland' ||
            process.env.WAYLAND_DISPLAY !== undefined)

        if (waylandSession) {
          warnings.push(
            'Linux Wayland session detected; Electron setBounds may be ignored by the compositor.'
          )
        }

        if (window === undefined) {
          return {
            applied: false,
            requestedTarget: placement.target,
            actualTarget: 'unresolved',
            fallbackUsed: false,
            targetHonored: false,
            message: 'No BrowserWindow was available for placement.',
            warnings
          }
        }

        const displays = screen.getAllDisplays()
        const primaryDisplay = screen.getPrimaryDisplay()
        const secondaryDisplay = displays.find((display) => display.id !== primaryDisplay.id)
        const currentBounds = window.getBounds()
        const requestedBounds = placement.bounds ?? {
          width: currentBounds.width,
          height: currentBounds.height
        }
        let targetDisplay = placement.target === 'primary' ? primaryDisplay : secondaryDisplay
        let fallbackUsed = false

        if (targetDisplay === undefined) {
          if (placement.fallback === 'unchanged') {
            return {
              applied: false,
              requestedTarget: placement.target,
              actualTarget: displayTargetName(
                screen.getDisplayMatching(currentBounds),
                primaryDisplay
              ),
              fallbackUsed: false,
              targetHonored: false,
              message:
                'No secondary display was found; leaving BrowserWindow bounds unchanged by fallback policy.',
              warnings
            }
          }

          targetDisplay = primaryDisplay
          fallbackUsed = true
        }

        const margin = 40
        const workArea = targetDisplay.workArea
        const nextBounds = {
          x: workArea.x + margin,
          y: workArea.y + margin,
          width: requestedBounds.width,
          height: requestedBounds.height
        }

        window.setBounds(nextBounds)
        const actualTarget = displayTargetName(
          screen.getDisplayMatching(window.getBounds()),
          primaryDisplay
        )
        const targetHonored = actualTarget === placement.target

        return {
          applied: true,
          requestedTarget: placement.target,
          actualTarget,
          fallbackUsed,
          targetHonored,
          message:
            `Placed BrowserWindow at ${nextBounds.x},${nextBounds.y} ` +
            `${nextBounds.width}x${nextBounds.height}.`,
          warnings
        }
      },
      request
    )

    log(
      `[window-placement] ${result.message} requested=${result.requestedTarget}; ` +
        `actual=${result.actualTarget}; fallbackUsed=${String(result.fallbackUsed)}; ` +
        `strict=${String(request.strict)}; targetHonored=${String(result.targetHonored)}.`
    )

    for (const warning of result.warnings) {
      log(`[window-placement] ${warning}`)
    }

    if (request.strict && (!result.applied || !result.targetHonored)) {
      throw new Error(
        `Strict window placement failed: requested=${result.requestedTarget}; ` +
          `actual=${result.actualTarget}; fallbackUsed=${String(result.fallbackUsed)}; ` +
          `applied=${String(result.applied)}.`
      )
    }
  } catch (error) {
    const message = formatUnknownError(error)
    log(`[window-placement] placement failed: ${message}`)

    if (request.strict) {
      throw error
    }
  }
}

function parseWindowPlacementRequest(
  env: NodeJS.ProcessEnv,
  log: (line: string) => void
): WindowPlacementRequest | undefined {
  const target = parseTarget(env[targetEnvVar], log)

  if (target === undefined) {
    return undefined
  }

  const fallback = parseFallback(env[fallbackEnvVar], log)
  const bounds = parseBounds(env[boundsEnvVar], log)
  const strict = isTruthy(env[strictEnvVar])

  return {
    target,
    fallback,
    strict,
    ...(bounds === undefined ? {} : { bounds })
  }
}

function parseTarget(
  value: string | undefined,
  log: (line: string) => void
): WindowPlacementTarget | undefined {
  if (value === undefined || value.length === 0) {
    return undefined
  }

  if (value === 'primary' || value === 'secondary' || value === 'none') {
    return value
  }

  log(
    `[window-placement] ignoring unsupported ${targetEnvVar}=${value}; expected primary, secondary, or none.`
  )
  return undefined
}

function parseFallback(
  value: string | undefined,
  log: (line: string) => void
): WindowPlacementFallback {
  if (value === undefined || value.length === 0) {
    return 'corner'
  }

  if (value === 'corner' || value === 'unchanged') {
    return value
  }

  log(
    `[window-placement] ignoring unsupported ${fallbackEnvVar}=${value}; expected corner or unchanged.`
  )
  return 'corner'
}

function parseBounds(
  value: string | undefined,
  log: (line: string) => void
): WindowPlacementRequest['bounds'] | undefined {
  if (value === undefined || value.length === 0) {
    return undefined
  }

  const match = /^(\d+)x(\d+)$/.exec(value)

  if (match === null) {
    log(`[window-placement] ignoring invalid ${boundsEnvVar}=${value}; expected WIDTHxHEIGHT.`)
    return undefined
  }

  const width = Number(match[1])
  const height = Number(match[2])

  if (!Number.isInteger(width) || !Number.isInteger(height) || width <= 0 || height <= 0) {
    log(
      `[window-placement] ignoring invalid ${boundsEnvVar}=${value}; dimensions must be positive.`
    )
    return undefined
  }

  return { width, height }
}

function isTruthy(value: string | undefined): boolean {
  return value === '1' || value === 'true' || value === 'yes'
}

function formatRequestedBounds(bounds: WindowPlacementRequest['bounds']): string {
  return bounds === undefined ? 'current' : `${bounds.width}x${bounds.height}`
}

function formatUnknownError(error: unknown): string {
  if (error instanceof Error) {
    return error.stack ?? error.message
  }

  return String(error)
}
