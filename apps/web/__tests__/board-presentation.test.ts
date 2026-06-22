import { describe, expect, it } from 'vitest';
import { clampAccent, spillColors } from '../app/components/spill-ui';
import type { RetroBoard } from '../app/lib/contracts';
import { columnSemantic } from '../app/retros/[retroId]/board-presentation';

function column(overrides: Partial<RetroBoard['columns'][number]>): RetroBoard['columns'][number] {
  return {
    id: 'column-1',
    retro_id: 'retro-1',
    column_key: 'custom',
    title: 'Custom',
    position: 0,
    accent_color: null,
    cards: [],
    ...overrides,
  };
}

describe('columnSemantic', () => {
  it('keeps the action semantic and color exclusive to action columns', () => {
    expect(columnSemantic(column({ column_key: 'actions', title: 'Actions' }))).toMatchObject({
      kind: 'action',
      color: spillColors.action,
    });

    expect(columnSemantic(column({ title: 'Love notes', accent_color: spillColors.action }))).toMatchObject({
      kind: 'neutral',
      color: spillColors.muted,
    });
  });

  it('does not classify action substrings as action columns', () => {
    expect(columnSemantic(column({ title: 'Satisfaction' }))).toMatchObject({
      kind: 'neutral',
      color: spillColors.muted,
    });
  });

  it('uses a neutral semantic for unmatched custom columns', () => {
    expect(columnSemantic(column({ title: 'Love notes' }))).toMatchObject({
      kind: 'neutral',
      color: spillColors.muted,
      label: 'love notes',
    });
  });

  it('preserves non-action saved colors on custom columns', () => {
    expect(columnSemantic(column({ title: 'Questions', accent_color: '#0f5f72' }))).toMatchObject({
      kind: 'neutral',
      color: '#0f5f72',
    });
  });

  it('does not clamp generated palette colors saved by board templates', () => {
    expect(columnSemantic(column({ title: 'Went well', accent_color: spillColors.well }))).toMatchObject({
      kind: 'well',
      color: spillColors.well,
    });
  });

  it('darkens a too-light saved color so white card text stays readable', () => {
    const result = columnSemantic(column({ title: 'Love notes', accent_color: '#ffd9e6' }));
    expect(result.color).toBe(clampAccent('#ffd9e6'));
    expect(result.color).not.toBe('#ffd9e6');
  });

  it('labels improvement columns as improve instead of wrong', () => {
    expect(columnSemantic(column({ title: 'To improve' }))).toMatchObject({
      kind: 'improve',
      color: spillColors.wrong,
      label: 'to improve',
    });
  });
});
