import { describe, expect, it } from 'vitest';
import { clampAccent, readableTextColor } from '../app/components/spill-ui';

describe('clampAccent', () => {
  it('leaves dark accents untouched', () => {
    expect(clampAccent('#0f5f72')).toBe('#0f5f72');
    expect(clampAccent('#8757b6')).toBe('#8757b6');
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
  it('uses white on dark accents and dark ink on light accents', () => {
    expect(readableTextColor('#0f5f72')).toBe('#ffffff');
    expect(readableTextColor('#ffffff')).toBe('#241a12');
  });
});
