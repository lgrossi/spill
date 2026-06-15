import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { CardAutosaveForm } from '../app/retros/[retroId]/card-autosave-form';

describe('CardAutosaveForm blur-save', () => {
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

  function setup({ withGif, text = '' }: { withGif: boolean; text?: string }) {
    render(
      <>
        <CardAutosaveForm action={() => {}}>
          {withGif ? <input name="gif_choice" type="hidden" value="{}" /> : null}
          <button data-intent-card-submit type="submit">save</button>
          <textarea defaultValue={text} name="body_text" />
        </CardAutosaveForm>
        <button type="button">outside</button>
        <div data-gif-overlay>
          <button type="button">in overlay</button>
        </div>
      </>,
    );
    return {
      textarea: screen.getByRole('textbox'),
      outside: screen.getByRole('button', { name: 'outside' }),
      inOverlay: screen.getByRole('button', { name: 'in overlay' }),
      save: screen.getByRole('button', { name: 'save' }),
    };
  }

  it('autosaves a gif-only card when focus leaves the form', () => {
    const { textarea, outside } = setup({ withGif: true });
    fireEvent.blur(textarea, { relatedTarget: outside });
    expect(requestSubmit).toHaveBeenCalledTimes(1);
  });

  it('autosaves a text card when focus leaves the form', () => {
    const { textarea, outside } = setup({ withGif: false, text: 'remember this' });
    fireEvent.blur(textarea, { relatedTarget: outside });
    expect(requestSubmit).toHaveBeenCalledTimes(1);
  });

  it('does not autosave an empty card', () => {
    const { textarea, outside } = setup({ withGif: false });
    fireEvent.blur(textarea, { relatedTarget: outside });
    expect(requestSubmit).not.toHaveBeenCalled();
  });

  it('does not autosave when focus moves into the gif overlay', () => {
    const { textarea, inOverlay } = setup({ withGif: true });
    fireEvent.blur(textarea, { relatedTarget: inOverlay });
    expect(requestSubmit).not.toHaveBeenCalled();
  });

  it('does not autosave when focus stays inside the form', () => {
    const { textarea, save } = setup({ withGif: true });
    fireEvent.blur(textarea, { relatedTarget: save });
    expect(requestSubmit).not.toHaveBeenCalled();
  });
});
