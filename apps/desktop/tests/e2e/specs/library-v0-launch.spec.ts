import { expect, test } from '../fixtures/libraryFixture'

test.describe('Library V0 launch smoke', () => {
  test('launches the built app with isolated user data and reaches Add Source', async ({
    electronApp,
    libraryV0
  }) => {
    await libraryV0.expectLaunchReady()
    await libraryV0.openAddSource()

    await expect(electronApp.close()).resolves.toMatchObject({
      graceful: true,
      killed: false,
      timedOut: false
    })
  })
})
