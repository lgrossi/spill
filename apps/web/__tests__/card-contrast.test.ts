import { describe, expect, it } from 'vitest';
import { clampAccent, readableCardTextColor, readableComposerFieldTextColor, readableTextColor } from '../app/components/spill-ui';

describe('clampAccent', () => {
  it('leaves dark accents untouched', () => {
    expect(clampAccent('#0f5f72')).toBe('#0f5f72');
    expect(clampAccent('#4d2f72')).toBe('#4d2f72');
  });

  it('darkens a too-light accent until white text is readable', () => {
    const clamped = clampAccent('#ffd9e6');
    expect(clamped).toBe('#87616e');
    expect(readableTextColor(clamped)).toBe('#ffffff');
  });

  it('tolerates shorthand and missing-hash input', () => {
    expect(clampAccent('fff')).not.toBe('#ffffff');
    expect(readableTextColor(clampAccent('fff'))).toBe('#ffffff');
  });

  it('falls back for malformed hex input', () => {
    expect(clampAccent('#zzzzzz')).toBe('#000000');
    expect(clampAccent('nope')).toBe('#000000');
  });
});

describe('readableTextColor', () => {
  it('chooses the higher-contrast foreground for each accent', () => {
    expect(readableTextColor('#0f5f72')).toBe('#ffffff');
    expect(readableTextColor('#cf4f4f')).toBe('#ffffff');
    expect(readableTextColor('#2f9469')).toBe('#241a12');
    expect(readableTextColor('#ffffff')).toBe('#241a12');
  });

  it('chooses card text against the whole rendered gradient', () => {
    expect(readableCardTextColor('#cf4f4f')).toBe('#ffffff');
    expect(readableCardTextColor('#2f9469')).toBe('#241a12');
  });

  it('chooses composer field text against the textarea overlay', () => {
    expect(readableComposerFieldTextColor('#2f9469')).toBe('#ffffff');
    expect(readableComposerFieldTextColor('#ffffff')).toBe('#241a12');
  });
});
