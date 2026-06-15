import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { IntentCardText } from '../app/components/intent-controls';

describe('IntentCardText autosave', () => {
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
      <>
        <form>
          {withGif ? <input name="gif_choice" type="hidden" value="{}" /> : null}
          <button data-intent-card-submit type="submit">save</button>
          <IntentCardText className="t" name="body_text" />
        </form>
        <button type="button">outside</button>
      </>,
    );
    return {
      textarea: screen.getByRole('textbox'),
      outside: screen.getByRole('button', { name: 'outside' }),
    };
  }

  it('autosaves a gif-only card when focus leaves the form (hidden gif_choice)', () => {
    const { textarea, outside } = setup({ withGif: true });
    fireEvent.blur(textarea, { relatedTarget: outside });
    expect(requestSubmit).toHaveBeenCalledTimes(1);
  });

  it('does not autosave an empty card with no gif', () => {
    const { textarea, outside } = setup({ withGif: false });
    fireEvent.blur(textarea, { relatedTarget: outside });
    expect(requestSubmit).not.toHaveBeenCalled();
  });

  it('lets the GIF picker own submission while it is open', () => {
    const { textarea, outside } = setup({ withGif: true });
    textarea.closest('form')!.dataset.gifPickerOpen = '1';
    fireEvent.blur(textarea, { relatedTarget: outside });
    expect(requestSubmit).not.toHaveBeenCalled();
  });
});
