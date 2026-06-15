import type { Locator, Page } from '@playwright/test'

import { testIds, type TestId } from './testIds'

export type LocatorRoot = Locator | Page

export function region(root: LocatorRoot, name: string | RegExp): Locator {
  return root.getByRole('region', { name })
}

export function group(root: LocatorRoot, name: string | RegExp): Locator {
  return root.getByRole('group', { name })
}

export function button(root: LocatorRoot, name: string | RegExp): Locator {
  return root.getByRole('button', { name })
}

export function labelledControl(root: LocatorRoot, name: string | RegExp): Locator {
  return root.getByLabel(name)
}

export function stableTestId(root: LocatorRoot, testId: TestId): Locator {
  return root.getByTestId(testIds[testId])
}
