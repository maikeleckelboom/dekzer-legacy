export type LibraryBoundaryHostState =
  | 'idle'
  | 'starting'
  | 'started'
  | 'stopping'
  | 'stopped'
  | 'failed'

export type LibraryBoundaryHostErrorCode =
  | 'invalidUserDataPath'
  | 'missingDevelopmentBinary'
  | 'packagedBinaryUnavailable'
  | 'stdioTransportStartupFailure'
  | 'alreadyStarted'
  | 'notStarted'
  | 'stopping'
  | 'stopped'

export type LibraryBoundaryHostErrorDetails = {
  readonly binaryPath?: string
  readonly binarySource?: string
  readonly executableName?: string
  readonly resourceRoot?: string
  readonly state?: LibraryBoundaryHostState
  readonly startupDiagnostics?: readonly string[]
  readonly startupSchemaDiagnostic?: string
  readonly userDataPath?: string
  readonly userDataSource?: string
}

export function isSchemaMismatchLine(line: string): boolean {
  const lower = line.toLowerCase()
  return (
    (lower.includes('schema') && lower.includes('malformed')) ||
    (lower.includes('schema') && lower.includes('does not match')) ||
    (lower.includes('schema') && lower.includes('incompatible'))
  )
}

export class LibraryBoundaryHostError extends Error {
  readonly source = 'desktopLibraryBoundaryHost'
  readonly code: LibraryBoundaryHostErrorCode
  readonly details: LibraryBoundaryHostErrorDetails

  constructor(
    code: LibraryBoundaryHostErrorCode,
    message: string,
    options: {
      readonly cause?: unknown
      readonly details?: LibraryBoundaryHostErrorDetails
    } = {}
  ) {
    super(message, { cause: options.cause })
    this.name = new.target.name
    this.code = code
    this.details = options.details ?? {}
  }
}
