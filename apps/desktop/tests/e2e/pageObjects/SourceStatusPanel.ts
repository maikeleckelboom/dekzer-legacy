import type { Locator } from '@playwright/test'

export class SourceStatusPanel {
  readonly root: Locator

  constructor(libraryRoot: Locator) {
    this.root = libraryRoot.getByLabel('Source status')
  }

  action(name: string | RegExp): Locator {
    return this.root.getByRole('button', { name })
  }
}
