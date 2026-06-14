export type SourceStatusInvalidationController = {
  readonly invalidateSource: (sourceId: string) => void
}

export type SourceStatusInvalidationReaders = {
  readonly sourceLifecycleRead: SourceStatusInvalidationController
  readonly integrityRead: SourceStatusInvalidationController
  readonly maintenanceRead: SourceStatusInvalidationController
  readonly activityRead: SourceStatusInvalidationController
}

export function invalidateSourceStatus(
  readers: SourceStatusInvalidationReaders,
  sourceId: string
): void {
  readers.sourceLifecycleRead.invalidateSource(sourceId)
  readers.integrityRead.invalidateSource(sourceId)
  readers.maintenanceRead.invalidateSource(sourceId)
  readers.activityRead.invalidateSource(sourceId)
}
