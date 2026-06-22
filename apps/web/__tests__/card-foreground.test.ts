import { describe, expect, it } from 'vitest';
import { readableCardControlColor, readableCardTextColor, spillColors } from '../app/components/spill-ui';

describe('readableCardTextColor', () => {
  it('chooses the higher-contrast card body foreground', () => {
    expect(readableCardTextColor(spillColors.well)).toBe('#241a12');
    expect(readableCardTextColor(spillColors.action)).toBe('#ffffff');
  });
});

describe('readableCardControlColor', () => {
  it('keeps control text readable on white card buttons', () => {
    expect(readableCardControlColor('#ffd9e6')).toBe('#241a12');
    expect(readableCardControlColor(spillColors.well)).toBe('#241a12');
    expect(readableCardControlColor(spillColors.action)).toBe(spillColors.action);
  });
});
