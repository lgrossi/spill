type ShutdownHook = {
  name: string;
  fn: () => Promise<void> | void;
  timeoutMs?: number;
};

export type InstallOpts = {
  drainMs?: number;
  onExit?: (code: number) => void;
  attach?: (signal: string, handler: () => void) => void;
};

const DEFAULT_DRAIN_MS = 5_000;
const DEFAULT_HOOK_TIMEOUT_MS = 5_000;

let shuttingDown = false;
const hooks: ShutdownHook[] = [];

export function isShuttingDown(): boolean {
  return shuttingDown;
}

export function onShutdown(hook: ShutdownHook): void {
  hooks.push(hook);
}

/** Resets module-level mutable state. Only for unit tests. */
export function __resetForTests(): void {
  shuttingDown = false;
  hooks.length = 0;
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function runWithTimeout(
  fn: () => Promise<void> | void,
  timeoutMs: number,
): Promise<void> {
  return Promise.race([
    Promise.resolve().then(fn),
    new Promise<void>((_, reject) =>
      setTimeout(
        () => reject(new Error(`timed out after ${timeoutMs}ms`)),
        timeoutMs,
      ),
    ),
  ]);
}

export function installSignalHandlers(opts: InstallOpts = {}): void {
  const drainMs =
    opts.drainMs ??
    (parseInt(process.env['SPILLIO_SHUTDOWN_DRAIN_MS'] ?? '', 10) || DEFAULT_DRAIN_MS);
  const onExit = opts.onExit ?? ((code: number) => process.exit(code));
  const attach =
    opts.attach ?? ((signal: string, handler: () => void) => process.on(signal, handler));

  const handler = (): void => {
    // Idempotent: ignore repeated signals once shutdown is in progress.
    if (shuttingDown) return;
    shuttingDown = true;

    void (async () => {
      await sleep(drainMs);

      for (const hook of hooks) {
        const timeout = hook.timeoutMs ?? DEFAULT_HOOK_TIMEOUT_MS;
        try {
          await runWithTimeout(hook.fn, timeout);
        } catch (err) {
          console.warn(
            `[shutdown] hook "${hook.name}" failed: ${err instanceof Error ? err.message : String(err)}`,
          );
        }
      }

      onExit(0);
    })();
  };

  attach('SIGTERM', handler);
  attach('SIGINT', handler);
}
