import type { App } from 'electron'
import {
  createLibraryBoundaryClient,
  type LibraryBoundaryClient
} from '@dekzer/library-boundary-client'
import {
  createLibraryBoundaryStdioTransport,
  type LibraryBoundaryStdioDiagnostic,
  type LibraryBoundaryStdioTransport
} from '@dekzer/library-boundary-stdio-transport'

import {
  resolveLibraryBoundaryHostConfig,
  resolveLibraryBoundaryStdioBinaryPath,
  type LibraryBoundaryHostConfig
} from './libraryBoundaryHostConfig'
import {
  LibraryBoundaryHostError,
  type LibraryBoundaryHostState
} from './libraryBoundaryHostErrors'

export type LibraryBoundaryHostLogger = {
  warn(message?: unknown, ...optionalParams: unknown[]): void
  error?(message?: unknown, ...optionalParams: unknown[]): void
}

export type CreateLibraryBoundaryHostOptions = {
  readonly app: Pick<App, 'getPath' | 'getAppPath'>
  readonly isDev: boolean
  readonly env?: NodeJS.ProcessEnv
  readonly platform?: NodeJS.Platform
  readonly resourcesPath?: string | null
  readonly logger?: LibraryBoundaryHostLogger
}

export class LibraryBoundaryHost {
  readonly #config: LibraryBoundaryHostConfig
  readonly #logger: LibraryBoundaryHostLogger
  #client: LibraryBoundaryClient | null = null
  #startPromise: Promise<LibraryBoundaryClient> | null = null
  #state: LibraryBoundaryHostState = 'idle'
  #stopPromise: Promise<void> | null = null
  #transport: LibraryBoundaryStdioTransport | null = null

  constructor(
    config: LibraryBoundaryHostConfig,
    logger: LibraryBoundaryHostLogger = console
  ) {
    this.#config = config
    this.#logger = logger
  }

  get config(): LibraryBoundaryHostConfig {
    return this.#config
  }

  get state(): LibraryBoundaryHostState {
    return this.#state
  }

  get hasStarted(): boolean {
    return (
      this.#state === 'starting' ||
      this.#state === 'started' ||
      this.#state === 'stopping'
    )
  }

  get client(): LibraryBoundaryClient {
    if (this.#client !== null && this.#state === 'started') {
      return this.#client
    }

    throw this.#stateErrorForClientAccess()
  }

  start(): Promise<LibraryBoundaryClient> {
    if (this.#state === 'started' || this.#state === 'starting') {
      return Promise.reject(
        new LibraryBoundaryHostError(
          'alreadyStarted',
          'Library boundary host has already been started.',
          { details: { state: this.#state } }
        )
      )
    }

    if (this.#state === 'stopping') {
      return Promise.reject(
        new LibraryBoundaryHostError(
          'stopping',
          'Library boundary host cannot start while it is stopping.',
          { details: { state: this.#state } }
        )
      )
    }

    if (this.#state === 'stopped' || this.#state === 'failed') {
      return Promise.reject(
        new LibraryBoundaryHostError(
          'stopped',
          'Library boundary host cannot restart after it has stopped.',
          { details: { state: this.#state } }
        )
      )
    }

    this.#state = 'starting'
    this.#startPromise = this.#start()
    return this.#startPromise
  }

  async stop(): Promise<void> {
    if (
      this.#state === 'idle' ||
      this.#state === 'stopped' ||
      this.#state === 'failed'
    ) {
      return
    }

    if (this.#stopPromise !== null) {
      return this.#stopPromise
    }

    this.#state = 'stopping'
    this.#stopPromise = this.#stop()
    return this.#stopPromise
  }

  async #start(): Promise<LibraryBoundaryClient> {
    try {
      const serverBinaryPath = resolveLibraryBoundaryStdioBinaryPath(
        this.#config.binaryPolicy
      )
      const transport = createLibraryBoundaryStdioTransport({
        serverBinaryPath,
        userDataPath: this.#config.userDataPath,
        environment: this.#config.environment,
        diagnostics: (diagnostic) => this.#handleDiagnostic(diagnostic)
      })
      const client = createLibraryBoundaryClient(transport)

      this.#transport = transport
      this.#client = client
      this.#state = 'started'
      return client
    } catch (cause) {
      this.#client = null
      this.#transport = null
      this.#startPromise = null

      if (cause instanceof LibraryBoundaryHostError) {
        this.#state = 'idle'
        throw cause
      }

      this.#state = 'failed'
      throw new LibraryBoundaryHostError(
        'stdioTransportStartupFailure',
        'Failed to start the library boundary stdio transport.',
        { cause }
      )
    }
  }

  async #stop(): Promise<void> {
    try {
      await this.#startPromise?.catch(() => undefined)
      await this.#transport?.close()
      this.#client = null
      this.#transport = null
      this.#startPromise = null
      this.#state = 'stopped'
    } catch (cause) {
      this.#state = 'failed'
      this.#stopPromise = null
      throw cause
    }
  }

  #handleDiagnostic(diagnostic: LibraryBoundaryStdioDiagnostic): void {
    this.#logger.warn(
      `[library-boundary-host] ${diagnostic.stream}: ${diagnostic.line}`
    )
  }

  #stateErrorForClientAccess(): LibraryBoundaryHostError {
    if (this.#state === 'stopping') {
      return new LibraryBoundaryHostError(
        'stopping',
        'Library boundary host client is unavailable while stopping.',
        { details: { state: this.#state } }
      )
    }

    if (this.#state === 'stopped' || this.#state === 'failed') {
      return new LibraryBoundaryHostError(
        'stopped',
        'Library boundary host client is unavailable after stop.',
        { details: { state: this.#state } }
      )
    }

    return new LibraryBoundaryHostError(
      'notStarted',
      'Library boundary host client is only available after start.',
      { details: { state: this.#state } }
    )
  }
}

export function createLibraryBoundaryHost(
  options: CreateLibraryBoundaryHostOptions
): LibraryBoundaryHost {
  return new LibraryBoundaryHost(
    resolveLibraryBoundaryHostConfig(options),
    options.logger
  )
}
