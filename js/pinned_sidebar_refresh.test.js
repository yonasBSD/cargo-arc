import { describe, expect, mock, test } from 'bun:test';
import { createPinnedSidebarRefresher } from './svg_script.js';

describe('createPinnedSidebarRefresher', () => {
  test('calls showNode with node ID and relations when node is pinned', () => {
    const relations = { incoming: [], outgoing: [] };
    const showNode = mock();
    const refresh = createPinnedSidebarRefresher(
      () => ({ type: 'node', id: 'crate_a' }),
      () => relations,
      showNode,
    );

    refresh();

    expect(showNode).toHaveBeenCalledWith('crate_a', relations);
  });

  test('does nothing when nothing is pinned', () => {
    const showNode = mock();
    const refresh = createPinnedSidebarRefresher(
      () => null,
      () => ({}),
      showNode,
    );

    refresh();

    expect(showNode).not.toHaveBeenCalled();
  });

  test('does nothing when pinned is undefined', () => {
    const showNode = mock();
    const refresh = createPinnedSidebarRefresher(
      () => undefined,
      () => ({}),
      showNode,
    );

    refresh();

    expect(showNode).not.toHaveBeenCalled();
  });

  test('does nothing when an arc is pinned', () => {
    const showNode = mock();
    const refresh = createPinnedSidebarRefresher(
      () => ({ type: 'arc', id: 'a-b' }),
      () => ({}),
      showNode,
    );

    refresh();

    expect(showNode).not.toHaveBeenCalled();
  });

  test('passes pinned node ID to collectRelations', () => {
    const collectRelations = mock(() => ({ incoming: [], outgoing: [] }));
    const refresh = createPinnedSidebarRefresher(
      () => ({ type: 'node', id: 'crate_x' }),
      collectRelations,
      () => {},
    );

    refresh();

    expect(collectRelations).toHaveBeenCalledWith('crate_x');
  });
});
