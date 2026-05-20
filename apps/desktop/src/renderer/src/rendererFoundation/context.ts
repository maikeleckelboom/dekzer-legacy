import type { InjectionKey } from 'vue'
import { inject, provide } from 'vue'

export function createContext<TContext extends object>(
  contextName: string
): {
  readonly provideContext: (context: TContext) => void
  readonly injectContext: () => TContext
} {
  const injectionKey: InjectionKey<TContext | undefined> = Symbol(`${contextName}Context`)

  function provideContext(context: TContext): void {
    provide(injectionKey, context)
  }

  function injectContext(): TContext {
    const context = inject(injectionKey, undefined)

    if (context === undefined) {
      throw new Error(`${contextName} context was injected outside of its provider.`)
    }

    return context
  }

  return {
    provideContext,
    injectContext
  }
}
