export async function register(): Promise<void> {
  if (process.env.NEXT_RUNTIME !== 'nodejs') return;

  const { installConsoleShim } = await import('./app/lib/console-shim');
  installConsoleShim();

  const { default: tracer } = await import('dd-trace');
  tracer.init({ logInjection: true });

  const { installSignalHandlers, onShutdown } = await import('./app/lib/shutdown');
  installSignalHandlers();
  onShutdown({
    name: 'tracer-flush',
    timeoutMs: 4000,
    fn: () =>
      new Promise<void>((resolve) => {
        (tracer as unknown as { flush: (cb: () => void) => void }).flush(resolve);
      }),
  });
}

export async function onRequestError(err: unknown): Promise<void> {
  const { default: tracer } = await import('dd-trace');
  const span = tracer.scope().active();
  if (span && err instanceof Error) {
    span.setTag('error', err);
  }
}
