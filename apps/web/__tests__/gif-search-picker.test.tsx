import { describe, it, expect, vi, beforeEach } from 'vitest';
import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';

const setQuery = vi.fn();
const loadMore = vi.fn();

vi.mock('../app/retros/[retroId]/gif-search-data', () => ({
  useGifSearch: () => ({
    degraded: false,
    hasMore: false,
    loadMore,
    loading: false,
    query: 'cat',
    results: [
      { id: 'gif-1', url: 'https://example.test/1.gif', preview_url: 'https://example.test/1p.gif', alt_text: 'a cat' },
    ],
    setQuery,
  }),
}));

import { GifSearchPicker } from '../app/retros/[retroId]/gif-search-picker';
import { GifDraftProvider, GifSelectedPreview } from '../app/retros/[retroId]/gif-draft';

function renderInForm() {
  const onSubmit = vi.fn((event: React.FormEvent) => event.preventDefault());
  const utils = render(
    <form onSubmit={onSubmit}>
      <GifDraftProvider>
        <GifSelectedPreview />
        <GifSearchPicker columnTitle="things" />
        <button data-intent-card-submit type="submit">save</button>
      </GifDraftProvider>
    </form>,
  );
  return { onSubmit, ...utils };
}

describe('GifSearchPicker', () => {
  beforeEach(() => {
    setQuery.mockReset();
    loadMore.mockReset();
  });

  it('stages the selected gif without submitting the card form', () => {
    const { onSubmit, container } = renderInForm();

    fireEvent.click(screen.getByRole('button', { name: /gif/i }));
    fireEvent.click(screen.getByRole('radio', { name: /Choose card GIF: a cat/i }));

    // Selecting must not submit: that previously created a duplicate standalone card.
    expect(onSubmit).not.toHaveBeenCalled();
    // The gif is staged onto the open form so the explicit ✓ / blur submit carries it.
    expect(container.querySelector('input[name="gif_choice"]')).not.toBeNull();
    // The overlay closes after selection.
    expect(screen.queryByRole('radio', { name: /Choose card GIF/i })).toBeNull();
  });
});
