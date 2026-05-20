import {
  libraryBoundaryHostStatusIpcChannels,
  type LibraryBoundaryHostStatus,
  type LibraryBoundaryHostStatusBinaryPolicy,
  type LibraryBoundaryHostStatusChangedCallback,
  type LibraryBoundaryHostStatusError,
  type LibraryBoundaryHostStatusState
} from '../shared/libraryBoundaryStatus'
import type { LibraryBoundaryHost } from './libraryBoundaryHost'
import type { LibraryBoundaryHostBinaryPolicy } from './libraryBoundaryHostConfig'
import {
  LibraryBoundaryHostError,
  type LibraryBoundaryHostErrorCode
} from './libraryBoundaryHostErrors'

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
  ipcMain.handle(libraryBoundaryHostStatusIpcChannels.getStatus, () => controller.getStatus())
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
    return {
      code: error.code,
      message: statusMessageForHostError(error.code)
    }
  }

  return {
    code: 'unknown',
    message: 'The library boundary host failed unexpectedly.'
  }
}

function statusMessageForHostError(code: LibraryBoundaryHostErrorCode): string {
  switch (code) {
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
