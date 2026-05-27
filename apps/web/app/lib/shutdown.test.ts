// @vitest-environment node
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  isShuttingDown,
  onShutdown,
  installSignalHandlers,
  __resetForTests,
} from './shutdown';

/** Wires up handlers via the attach seam and returns a trigger function per signal. */
function setup(opts: {
  drainMs?: number;
  onExit?: (code: number) => void;
}): Record<string, () => void> {
  const handlers: Record<string, () => void> = {};
  installSignalHandlers({
    drainMs: opts.drainMs ?? 0,
    onExit: opts.onExit ?? vi.fn(),
    attach: (signal, handler) => {
      handlers[signal] = handler;
    },
  });
  return handlers;
}

describe('shutdown coordinator', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    __resetForTests();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('isShuttingDown() starts false and flips synchronously when signal fires', async () => {
    const signals = setup({});
    expect(isShuttingDown()).toBe(false);
    signals['SIGTERM']!();
    expect(isShuttingDown()).toBe(true);
    await vi.runAllTimersAsync();
  });

  it('runs hooks in registration order and calls exit 0', async () => {
    const order: string[] = [];
    const onExit = vi.fn();
    const signals = setup({ onExit });

    onShutdown({ name: 'first', fn: () => { order.push('first'); } });
    onShutdown({ name: 'second', fn: () => { order.push('second'); } });

    signals['SIGTERM']!();
    await vi.runAllTimersAsync();

    expect(order).toEqual(['first', 'second']);
    expect(onExit).toHaveBeenCalledWith(0);
  });

  it('a hook that throws does not prevent subsequent hooks from running', async () => {
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => undefined);
    const ran: string[] = [];
    const onExit = vi.fn();
    const signals = setup({ onExit });

    onShutdown({
      name: 'bad-hook',
      fn: () => { throw new Error('boom'); },
    });
    onShutdown({ name: 'good-hook', fn: () => { ran.push('good-hook'); } });

    signals['SIGTERM']!();
    await vi.runAllTimersAsync();

    expect(ran).toEqual(['good-hook']);
    expect(warnSpy).toHaveBeenCalledWith(expect.stringContaining('bad-hook'));
    expect(warnSpy).toHaveBeenCalledWith(expect.stringContaining('boom'));
    expect(onExit).toHaveBeenCalledWith(0);
    warnSpy.mockRestore();
  });

  it('enforces per-hook timeout and subsequent hooks still run', async () => {
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => undefined);
    const ran: string[] = [];
    const onExit = vi.fn();
    const signals = setup({ onExit });

    onShutdown({
      name: 'slow-hook',
      timeoutMs: 100,
      // Never resolves
      fn: () => new Promise<void>(() => undefined),
    });
    onShutdown({ name: 'after-slow', fn: () => { ran.push('after-slow'); } });

    signals['SIGTERM']!();
    await vi.runAllTimersAsync();

    expect(ran).toEqual(['after-slow']);
    expect(warnSpy).toHaveBeenCalledWith(expect.stringContaining('slow-hook'));
    expect(warnSpy).toHaveBeenCalledWith(expect.stringContaining('timed out'));
    expect(onExit).toHaveBeenCalledWith(0);
    warnSpy.mockRestore();
  });

  it('SIGINT is wired the same as SIGTERM', async () => {
    const onExit = vi.fn();
    const signals = setup({ onExit });

    signals['SIGINT']!();
    expect(isShuttingDown()).toBe(true);
    await vi.runAllTimersAsync();
    expect(onExit).toHaveBeenCalledWith(0);
  });

  it('second signal while already shutting down is a no-op', async () => {
    const onExit = vi.fn();
    const signals = setup({ onExit });

    onShutdown({ name: 'hook', fn: vi.fn() });

    signals['SIGTERM']!();
    signals['SIGTERM']!(); // duplicate
    signals['SIGINT']!();  // another duplicate via different signal

    await vi.runAllTimersAsync();

    // exit called exactly once despite three signal fires
    expect(onExit).toHaveBeenCalledTimes(1);
    expect(onExit).toHaveBeenCalledWith(0);
  });
});
