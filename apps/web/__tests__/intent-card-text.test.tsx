import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { IntentCardText } from '../app/components/intent-controls';

describe('IntentCardText Enter-to-submit', () => {
  let requestSubmit: ReturnType<typeof vi.fn>;
  let original: typeof HTMLFormElement.prototype.requestSubmit;

  beforeEach(() => {
    requestSubmit = vi.fn();
    original = HTMLFormElement.prototype.requestSubmit;
    HTMLFormElement.prototype.requestSubmit = requestSubmit as unknown as typeof HTMLFormElement.prototype.requestSubmit;
  });

  afterEach(() => {
    HTMLFormElement.prototype.requestSubmit = original;
  });

  function setup({ withGif }: { withGif: boolean }) {
    render(
      <form>
        {withGif ? <input name="gif_choice" type="hidden" value="{}" /> : null}
        <button data-intent-card-submit type="submit">save</button>
        <IntentCardText className="t" name="body_text" />
      </form>,
    );
    return { textarea: screen.getByRole('textbox') as HTMLTextAreaElement };
  }

  it('submits on Enter when the card has text', () => {
    const { textarea } = setup({ withGif: false });
    fireEvent.change(textarea, { target: { value: 'ship it' } });
    fireEvent.keyDown(textarea, { key: 'Enter' });
    expect(requestSubmit).toHaveBeenCalledTimes(1);
  });

  it('submits on Enter for a gif-only card (hidden gif_choice)', () => {
    const { textarea } = setup({ withGif: true });
    fireEvent.keyDown(textarea, { key: 'Enter' });
    expect(requestSubmit).toHaveBeenCalledTimes(1);
  });

  it('does not submit on Enter when the card is empty', () => {
    const { textarea } = setup({ withGif: false });
    fireEvent.keyDown(textarea, { key: 'Enter' });
    expect(requestSubmit).not.toHaveBeenCalled();
  });

  it('does not submit on Shift+Enter (newline)', () => {
    const { textarea } = setup({ withGif: false });
    fireEvent.change(textarea, { target: { value: 'line one' } });
    fireEvent.keyDown(textarea, { key: 'Enter', shiftKey: true });
    expect(requestSubmit).not.toHaveBeenCalled();
  });
});
