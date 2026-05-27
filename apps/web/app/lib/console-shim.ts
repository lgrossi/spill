// Replaces global console.* with writers that emit one JSON line per call,
// mapped to the GCP Cloud Logging structured payload (severity + message + stack_trace).
// Must be installed before dd-trace and any Next framework code.
import { format } from 'node:util';

declare global {
  // eslint-disable-next-line no-var
  var __spillioConsoleShimInstalled: boolean | undefined;
}

type Severity = 'DEBUG' | 'INFO' | 'WARNING' | 'ERROR' | 'CRITICAL';

function emit(severity: Severity, args: unknown[], stack?: string): void {
  const message =
    args.length === 1 && typeof args[0] === 'string'
      ? args[0]
      : format(...(args as [unknown, ...unknown[]]));
  const payload: Record<string, unknown> = {
    severity,
    message,
    time: new Date().toISOString(),
  };
  if (stack) payload['stack_trace'] = stack;
  process.stdout.write(JSON.stringify(payload) + '\n');
}

export function installConsoleShim(): void {
  if (global.__spillioConsoleShimInstalled) return;
  global.__spillioConsoleShimInstalled = true;

  console.log = (...args: unknown[]) => emit('INFO', args);
  console.info = (...args: unknown[]) => emit('INFO', args);
  console.warn = (...args: unknown[]) => emit('WARNING', args);
  console.error = (...args: unknown[]) => emit('ERROR', args);
  console.trace = (...args: unknown[]) => emit('DEBUG', args, new Error().stack);

  process.on('uncaughtException', (err: Error) => {
    emit('CRITICAL', [err.message], err.stack);
    process.exit(1);
  });

  process.on('unhandledRejection', (reason: unknown) => {
    const message = reason instanceof Error ? reason.message : String(reason);
    const stack = reason instanceof Error ? reason.stack : undefined;
    emit('CRITICAL', [message], stack);
  });
}
