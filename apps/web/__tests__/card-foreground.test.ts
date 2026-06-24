import { describe, expect, it } from 'vitest';
import { readableCardControlColor, readableCardTextColor, spillColors } from '../app/components/spill-ui';

describe('readableCardTextColor', () => {
  it('uses white text on saturated mid-tone brand colors (L* < 60)', () => {
    // Mollie green (L* ~54) and red (L* ~52) used to pick dark on raw WCAG
    // contrast but read muddy. With the L*-threshold picker they flip to
    // white -- the visually cleaner choice across all three real cards in
    // the user-reported screenshot.
    expect(readableCardTextColor(spillColors.well)).toBe('#ffffff');
    expect(readableCardTextColor(spillColors.wrong)).toBe('#ffffff');
    // Purple action and pure bright blue stay white (unchanged behavior).
    expect(readableCardTextColor(spillColors.action)).toBe('#ffffff');
    expect(readableCardTextColor('#0078fc')).toBe('#ffffff');
  });

  it('uses the soft brand ink on truly light backgrounds (L* >= 75)', () => {
    // Cream paper (L* ~92) and other near-white cards have generous
    // contrast headroom -- use --fg-2 so the dark text matches the soft
    // brown the rest of the UI uses for body copy on paper.
    expect(readableCardTextColor(spillColors.paper)).toBe(spillColors.inkSoft);
    expect(readableCardTextColor('#f5d99f')).toBe(spillColors.inkSoft);
  });

  it('uses the primary brand ink on borderline-light backgrounds (60 <= L* < 75)', () => {
    // Mood amber (L* ~62) lands in the borderline band where the softer
    // --fg-2 ink would fall under AA for normal-weight body text. Use the
    // stronger --fg ink to keep contrast safe without dropping back to
    // pure black.
    expect(readableCardTextColor(spillColors.mood)).toBe(spillColors.ink);
  });

  it('never returns the orphan #241a12 literal -- everything routes through the spillColors tokens', () => {
    for (const hex of ['#000000', '#ffffff', '#888888', spillColors.well, spillColors.paper, '#cf8a3f']) {
      expect(readableCardTextColor(hex)).not.toBe('#241a12');
    }
  });
});

describe('readableCardControlColor', () => {
  it('keeps control text readable on white card buttons', () => {
    expect(readableCardControlColor('#ffd9e6')).toBe(spillColors.ink);
    expect(readableCardControlColor(spillColors.well)).toBe(spillColors.ink);
    expect(readableCardControlColor(spillColors.action)).toBe(spillColors.action);
  });
});
