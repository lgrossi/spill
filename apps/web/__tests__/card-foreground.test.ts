import { describe, expect, it } from 'vitest';
import { readableCardTextColor, spillColors } from '../app/components/spill-ui';

describe('readableCardTextColor', () => {
  it('chooses the higher-contrast card body foreground', () => {
    expect(readableCardTextColor(spillColors.well)).toBe('#241a12');
    expect(readableCardTextColor(spillColors.action)).toBe('#ffffff');
  });
});
