import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { CardAutosaveForm } from '../app/retros/[retroId]/card-autosave-form';

const flush = () => new Promise((resolve) => setTimeout(resolve, 0));

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
          <input aria-label="overlay field" />
        </div>
      </>,
    );
    return {
      textarea: screen.getByRole('textbox', { name: '' }) as HTMLTextAreaElement,
      outside: screen.getByRole('button', { name: 'outside' }),
      overlay: screen.getByLabelText('overlay field'),
      save: screen.getByRole('button', { name: 'save' }),
    };
  }

  // The handler reads document.activeElement after a tick, so move focus first.
  async function leaveTo(textarea: HTMLElement, target: HTMLElement) {
    target.focus();
    fireEvent.blur(textarea);
    await flush();
  }

  it('autosaves a gif-only card when focus leaves the form', async () => {
    const { textarea, outside } = setup({ withGif: true });
    await leaveTo(textarea, outside);
    expect(requestSubmit).toHaveBeenCalledTimes(1);
  });

  it('autosaves a text card when focus leaves the form', async () => {
    const { textarea, outside } = setup({ withGif: false, text: 'remember this' });
    await leaveTo(textarea, outside);
    expect(requestSubmit).toHaveBeenCalledTimes(1);
  });

  it('does not autosave an empty card', async () => {
    const { textarea, outside } = setup({ withGif: false });
    await leaveTo(textarea, outside);
    expect(requestSubmit).not.toHaveBeenCalled();
  });

  it('does not autosave a non-empty card when opening the gif picker (focus into overlay)', async () => {
    const { textarea, overlay } = setup({ withGif: false, text: 'has text' });
    await leaveTo(textarea, overlay);
    expect(requestSubmit).not.toHaveBeenCalled();
  });

  it('does not autosave when focus stays inside the form', async () => {
    const { textarea, save } = setup({ withGif: true });
    await leaveTo(textarea, save);
    expect(requestSubmit).not.toHaveBeenCalled();
  });

  it('saves synchronously when focus moves to an outside element (survives an unmounting click)', () => {
    const { textarea, outside } = setup({ withGif: false, text: 'draft' });
    // relatedTarget is a known outside element: do not wait for the timer, because
    // the click may unmount the editor before it would fire.
    fireEvent.blur(textarea, { relatedTarget: outside });
    expect(requestSubmit).toHaveBeenCalledTimes(1);
  });

  it('does not save synchronously when focus moves into the gif overlay', () => {
    const { textarea, overlay } = setup({ withGif: false, text: 'has text' });
    fireEvent.blur(textarea, { relatedTarget: overlay });
    expect(requestSubmit).not.toHaveBeenCalled();
  });
});
