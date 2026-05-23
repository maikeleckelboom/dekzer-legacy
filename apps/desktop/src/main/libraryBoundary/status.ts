import {
  hostStatusChannels,
  type LibraryBoundaryHostStatus,
  type LibraryBoundaryHostStatusBinaryPolicy,
  type LibraryBoundaryHostStatusChangedCallback,
  type LibraryBoundaryHostStatusError,
  type LibraryBoundaryHostStatusState
} from '../../shared/libraryBoundary/status'
import type { LibraryBoundaryHost } from './host'
import type { LibraryBoundaryHostBinaryPolicy } from './config'
import { LibraryBoundaryHostError, type LibraryBoundaryHostErrorCode } from './errors'

export type LibraryBoundaryHostStatusLogger = {
  error(message?: unknown, ...optionalParams: unknown[]): void
}

export type LibraryBoundaryHostStatusIpcMain = {
  handle(
    channel: string,
    listener: (event: unknown, ...args: readonly unknown[]) => LibraryBoundaryHostStatus
  ): void
}

export class LibraryBoundaryHostStatusController {
  readonly #host: LibraryBoundaryHost
  readonly #listeners = new Set<LibraryBoundaryHostStatusChangedCallback>()
  readonly #logger: LibraryBoundaryHostStatusLogger
  #lastError: LibraryBoundaryHostStatusError | undefined

  constructor(host: LibraryBoundaryHost, logger: LibraryBoundaryHostStatusLogger = console) {
    this.#host = host
    this.#logger = logger
  }

  get hasStarted(): boolean {
    return this.#host.hasStarted
  }

  getStatus(): LibraryBoundaryHostStatus {
    return createLibraryBoundaryHostStatus(this.#host, this.#lastError)
  }

  onStatusChanged(callback: LibraryBoundaryHostStatusChangedCallback): () => void {
    this.#listeners.add(callback)
    return () => {
      this.#listeners.delete(callback)
    }
  }

  async start(): Promise<void> {
    if (this.#host.hasStarted) {
      this.#publish()
      return
    }

    this.#lastError = undefined
    const started = this.#host.start()
    this.#publish()

    try {
      await started
      this.#lastError = undefined
      this.#publish()
    } catch (error: unknown) {
      this.#lastError = createLibraryBoundaryHostStatusError(error)
      this.#logger.error('[library-boundary-host] failed to start', error)
      this.#publish()
    }
  }

  async stop(): Promise<void> {
    if (!this.#host.hasStarted) {
      return
    }

    const stopped = this.#host.stop()
    this.#publish()

    try {
      await stopped
      this.#publish()
    } catch (error: unknown) {
      this.#lastError = createLibraryBoundaryHostStatusError(error)
      this.#logger.error('[library-boundary-host] failed to stop cleanly', error)
      this.#publish()
      throw error
    }
  }

  #publish(): void {
    const status = this.getStatus()
    for (const listener of this.#listeners) {
      listener(status)
    }
  }
}

export function registerLibraryBoundaryHostStatusIpc(
  ipcMain: LibraryBoundaryHostStatusIpcMain,
  controller: LibraryBoundaryHostStatusController
): void {
  ipcMain.handle(hostStatusChannels.getStatus, () => controller.getStatus())
}

export function createLibraryBoundaryHostStatus(
  host: LibraryBoundaryHost,
  lastError?: LibraryBoundaryHostStatusError
): LibraryBoundaryHostStatus {
  return {
    state: projectState(host.state, lastError),
    environment: host.config.environment,
    binaryPolicy: projectBinaryPolicy(host.config.binaryPolicy),
    lastError: lastError ?? null
  }
}

function projectState(
  state: LibraryBoundaryHostStatusState,
  lastError: LibraryBoundaryHostStatusError | undefined
): LibraryBoundaryHostStatusState {
  if (lastError !== undefined && state === 'idle') {
    return 'failed'
  }

  return state
}

function projectBinaryPolicy(
  policy: LibraryBoundaryHostBinaryPolicy
): LibraryBoundaryHostStatusBinaryPolicy {
  if (policy.kind === 'developmentBinary') {
    return {
      kind: policy.kind,
      source: policy.source
    }
  }

  return {
    kind: policy.kind,
    executableName: policy.executableName
  }
}

function createLibraryBoundaryHostStatusError(error: unknown): LibraryBoundaryHostStatusError {
  if (error instanceof LibraryBoundaryHostError) {
    const detail = extractHostErrorCauseDetail(error)
    return {
      code: error.code,
      message: statusMessageForHostError(error.code),
      ...(detail === undefined ? {} : { detail })
    }
  }

  const detail = error instanceof Error ? truncateFirstLine(error.message) : undefined
  return {
    code: 'unknown',
    message: 'The library boundary host failed unexpectedly.',
    ...(detail === undefined ? {} : { detail })
  }
}

function extractHostErrorCauseDetail(error: LibraryBoundaryHostError): string | undefined {
  const cause = error.cause

  if (cause instanceof Error) {
    return truncateFirstLine(cause.message)
  }

  if (typeof cause === 'string') {
    return truncateFirstLine(cause)
  }

  return undefined
}

function truncateFirstLine(text: string): string | undefined {
  const firstLine = text.split('\n')[0]?.trim()

  if (firstLine === undefined || firstLine.length === 0) {
    return undefined
  }

  return firstLine.length <= 200 ? firstLine : `${firstLine.slice(0, 197)}...`
}

function statusMessageForHostError(code: LibraryBoundaryHostErrorCode): string {
  switch (code) {
    case 'invalidUserDataPath':
      return 'The library boundary user data path is invalid.'
    case 'missingDevelopmentBinary':
      return 'The development library boundary stdio binary is missing.'
    case 'packagedBinaryUnavailable':
      return 'The library boundary stdio binary is not packaged with the desktop app yet.'
    case 'stdioTransportStartupFailure':
      return 'Failed to start the library boundary stdio transport.'
    case 'alreadyStarted':
      return 'The library boundary host has already been started.'
    case 'notStarted':
      return 'The library boundary host has not been started.'
    case 'stopping':
      return 'The library boundary host is stopping.'
    case 'stopped':
      return 'The library boundary host is stopped.'
  }
}
