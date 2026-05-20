import type { InjectionKey } from 'vue'
import { inject, provide } from 'vue'

const missingContext = Symbol('missing renderer context')

export type RendererContext<TContext> = {
  readonly provide: (context: TContext) => TContext
  readonly inject: () => TContext
}

export function createRequiredContext<TContext>(options: {
  readonly contextName: string
  readonly providerName: string
}): RendererContext<TContext> {
  const injectionKey: InjectionKey<TContext> = Symbol(options.contextName)

  function provideContext(context: TContext): TContext {
    provide(injectionKey, context)
    return context
  }

  function injectContext(): TContext {
    const context = inject(
      injectionKey as InjectionKey<TContext | typeof missingContext>,
      missingContext
    )

    if (context === missingContext) {
      throw new Error(
        `Missing required renderer context "${options.contextName}". Render this component inside "${options.providerName}".`
      )
    }

    return context
  }

  return {
    provide: provideContext,
    inject: injectContext
  }
}
