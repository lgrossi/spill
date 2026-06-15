// Shared definition of "this card has something worth saving" plus the canonical
// way to submit a card form. Used by the form-level autosave and the textarea's
// Enter-to-submit so every entry point agrees on one rule.
export function cardFormHasContent(form: HTMLFormElement): boolean {
  const text = form.querySelector<HTMLTextAreaElement>('textarea[name="body_text"]')?.value.trim();
  const hasGif = Boolean(form.querySelector<HTMLInputElement>('input[name="gif_choice"]'));
  const existingGif = form
    .querySelector<HTMLInputElement>('input[name="existing_gif_url"]')
    ?.value.trim();
  return Boolean(text || hasGif || existingGif);
}

export function requestCardSubmit(form: HTMLFormElement): void {
  const submitter = form.querySelector<HTMLButtonElement>("[data-intent-card-submit]");
  form.requestSubmit(submitter ?? undefined);
}
