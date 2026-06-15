import '@testing-library/jest-dom';

// jsdom lacks these observers; the GIF overlay constructs both on mount.
class NoopObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
}

globalThis.ResizeObserver ??= NoopObserver as unknown as typeof ResizeObserver;
globalThis.IntersectionObserver ??= NoopObserver as unknown as typeof IntersectionObserver;
