import { describe, expect, it, vi } from 'vitest';

const { createRetroMock, redirectMock } = vi.hoisted(() => ({
  createRetroMock: vi.fn(),
  redirectMock: vi.fn(),
}));

vi.mock('../app/lib/api', () => ({
  createRetro: createRetroMock,
}));

vi.mock('next/navigation', () => ({
  redirect: redirectMock,
}));

import { createRetroCommand } from '../app/lib/commands/retro-commands';

describe('createRetroCommand', () => {
  it('sends the form-default per_column reveal mode explicitly', async () => {
    createRetroMock.mockResolvedValue({ retro: { id: 'retro-1' } });
    const formData = new FormData();
    formData.set('title', 'Sprint retro');
    formData.set('template', 'standard');

    await createRetroCommand(formData);

    expect(createRetroMock).toHaveBeenCalledWith(expect.objectContaining({
      reveal_mode: 'per_column',
    }));
    expect(redirectMock).toHaveBeenCalledWith('/retros/retro-1');
  });
});
