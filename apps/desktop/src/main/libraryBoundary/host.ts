import type { App } from 'electron'
import type {
  LibraryBoundaryClient,
  LibraryBoundaryTransport
} from '@dekzer/library-boundary-client'
import type {
  LibraryBoundaryStdioDiagnostic,
  LibraryBoundaryStdioTransportOptions
} from '@dekzer/library-boundary-stdio-transport'

import {
  resolveLibraryBoundaryHostConfig,
  resolveLibraryBoundaryStdioBinaryPath,
  type LibraryBoundaryHostConfig
} from './config'
import { LibraryBoundaryHostError, type LibraryBoundaryHostState } from './errors'

const MAX_STARTUP_DIAGNOSTIC_LINES = 8

export type LibraryBoundaryHostLogger = {
  warn(message?: unknown, ...optionalParams: unknown[]): void
  error?(message?: unknown, ...optionalParams: unknown[]): void
}

export type LibraryBoundaryHostClient = LibraryBoundaryClient

export type LibraryBoundaryHostTransport = Omit<LibraryBoundaryTransport, 'close'> & {
  readonly ready: Promise<void>
  close(): Promise<void>
}

export type { LibraryBoundaryStdioDiagnostic }

export type LibraryBoundaryHostTransportOptions = LibraryBoundaryStdioTransportOptions

export type LibraryBoundaryHostTransportFactory = (
  options: LibraryBoundaryHostTransportOptions
) => LibraryBoundaryHostTransport

export type LibraryBoundaryHostClientFactory = (
  transport: LibraryBoundaryHostTransport
) => LibraryBoundaryHostClient

export type LibraryBoundaryHostDependencies = {
  readonly createTransport?: LibraryBoundaryHostTransportFactory
  readonly createClient?: LibraryBoundaryHostClientFactory
  readonly resolveStdioBinaryPath?: typeof resolveLibraryBoundaryStdioBinaryPath
}

export type CreateLibraryBoundaryHostOptions = {
  readonly app: Pick<App, 'getPath' | 'getAppPath'>
  readonly isDev: boolean
  readonly env?: NodeJS.ProcessEnv
  readonly platform?: NodeJS.Platform
  readonly resourcesPath?: string
  readonly logger?: LibraryBoundaryHostLogger
}

export class LibraryBoundaryHost {
  readonly #config: LibraryBoundaryHostConfig
  readonly #dependencies: LibraryBoundaryHostDependencies
  readonly #logger: LibraryBoundaryHostLogger
  #client: LibraryBoundaryHostClient | undefined
  #startPromise: Promise<LibraryBoundaryHostClient> | undefined
  #state: LibraryBoundaryHostState = 'idle'
  #stopPromise: Promise<void> | undefined
  #startupDiagnostics: string[] = []
  #transport: LibraryBoundaryHostTransport | undefined

  constructor(
    config: LibraryBoundaryHostConfig,
    logger: LibraryBoundaryHostLogger = console,
    dependencies: LibraryBoundaryHostDependencies = {}
  ) {
    this.#config = config
    this.#dependencies = dependencies
    this.#logger = logger
  }

  get config(): LibraryBoundaryHostConfig {
    return this.#config
  }

  get state(): LibraryBoundaryHostState {
    return this.#state
  }

  get hasStarted(): boolean {
    return this.#state === 'starting' || this.#state === 'started' || this.#state === 'stopping'
  }

  get client(): LibraryBoundaryHostClient {
    if (this.#client !== undefined && this.#state === 'started') {
      return this.#client
    }

    throw this.#stateErrorForClientAccess()
  }

  start(): Promise<LibraryBoundaryHostClient> {
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
    this.#startupDiagnostics = []
    this.#startPromise = this.#start()
    return this.#startPromise
  }

  async stop(): Promise<void> {
    if (this.#state === 'idle' || this.#state === 'stopped' || this.#state === 'failed') {
      return
    }

    if (this.#stopPromise !== undefined) {
      return this.#stopPromise
    }

    this.#state = 'stopping'
    this.#stopPromise = this.#stop()
    return this.#stopPromise
  }

  async #start(): Promise<LibraryBoundaryHostClient> {
    try {
      const resolveStdioBinaryPath =
        this.#dependencies.resolveStdioBinaryPath ?? resolveLibraryBoundaryStdioBinaryPath
      const serverBinaryPath = resolveStdioBinaryPath(this.#config.binaryPolicy)
      const createTransport = await this.#resolveTransportFactory()
      const transport = createTransport({
        serverBinaryPath,
        userDataPath: this.#config.storageEnvironment.userDataPath,
        environment: this.#config.environment,
        diagnostics: (diagnostic) => this.#handleDiagnostic(diagnostic)
      })

      this.#transport = transport
      await transport.ready

      const createClient = await this.#resolveClientFactory()
      const client = createClient(transport)

      this.#client = client
      this.#state = 'started'
      this.#startupDiagnostics = []
      return client
    } catch (cause) {
      const startupDiagnostics = this.#startupDiagnostics.slice()
      await this.#transport?.close().catch(() => undefined)
      this.#client = undefined
      this.#transport = undefined
      this.#startPromise = undefined

      if (cause instanceof LibraryBoundaryHostError) {
        this.#state = 'idle'
        throw cause
      }

      this.#state = 'failed'
      throw new LibraryBoundaryHostError(
        'stdioTransportStartupFailure',
        'Failed to start the library boundary stdio transport.',
        {
          cause,
          details: startupDiagnostics.length === 0 ? {} : { startupDiagnostics }
        }
      )
    }
  }

  async #resolveTransportFactory(): Promise<LibraryBoundaryHostTransportFactory> {
    if (this.#dependencies.createTransport !== undefined) {
      return this.#dependencies.createTransport
    }

    const module = await import('@dekzer/library-boundary-stdio-transport')
    return module.createLibraryBoundaryStdioTransport
  }

  async #resolveClientFactory(): Promise<LibraryBoundaryHostClientFactory> {
    if (this.#dependencies.createClient !== undefined) {
      return this.#dependencies.createClient
    }

    const module = await import('@dekzer/library-boundary-client')
    return module.createLibraryBoundaryClient
  }

  async #stop(): Promise<void> {
    try {
      await this.#startPromise?.catch(() => undefined)
      await this.#transport?.close()
      this.#client = undefined
      this.#transport = undefined
      this.#startPromise = undefined
      this.#state = 'stopped'
    } catch (cause) {
      this.#state = 'failed'
      this.#stopPromise = undefined
      throw cause
    }
  }

  #handleDiagnostic(diagnostic: LibraryBoundaryStdioDiagnostic): void {
    if (this.#state === 'starting' && diagnostic.stream === 'stderr') {
      this.#startupDiagnostics.push(diagnostic.line)
      if (this.#startupDiagnostics.length > MAX_STARTUP_DIAGNOSTIC_LINES) {
        this.#startupDiagnostics.splice(
          0,
          this.#startupDiagnostics.length - MAX_STARTUP_DIAGNOSTIC_LINES
        )
      }
    }

    this.#logger.warn(`[library-boundary-host] ${diagnostic.stream}: ${diagnostic.line}`)
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
  return new LibraryBoundaryHost(resolveLibraryBoundaryHostConfig(options), options.logger)
}
