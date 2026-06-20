import { describe, expect, it } from 'vitest';
import { clampAccent, readableTextColor } from '../app/components/spill-ui';

describe('clampAccent', () => {
  it('leaves dark accents untouched', () => {
    expect(clampAccent('#0f5f72')).toBe('#0f5f72');
    expect(clampAccent('#2f9469')).toBe('#2f9469');
  });

  it('darkens a too-light accent until white text is readable', () => {
    const clamped = clampAccent('#ffd9e6');
    expect(clamped).not.toBe('#ffd9e6');
    // A clamped accent must yield white as the readable text color.
    expect(readableTextColor(clamped)).toBe('#ffffff');
  });

  it('tolerates shorthand and missing-hash input', () => {
    expect(clampAccent('fff')).not.toBe('#ffffff');
    expect(readableTextColor(clampAccent('fff'))).toBe('#ffffff');
  });
});

describe('readableTextColor', () => {
  it('uses white on dark accents and dark ink on light accents', () => {
    expect(readableTextColor('#0f5f72')).toBe('#ffffff');
    expect(readableTextColor('#ffffff')).toBe('#241a12');
  });
});
