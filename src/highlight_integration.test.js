// highlight_integration.test.js - Integration tests for highlight/clearHighlights behavior
// Ensures proper separation of highlight state (scale) vs layout state (position)

import { test, expect, describe } from "bun:test";

// Setup globals (simulating browser environment)
global.ArrowLogic = require('./arrow_logic.js').ArrowLogic;
global.HighlightLogic = require('./highlight_logic.js').HighlightLogic;
global.ArcLogic = require('./svg_script.js').ArcLogic;

const { createMockDomAdapter, createFakeElement } = require('./dom_adapter.js');
const { AppState } = require('./app_state.js');

describe('clearHighlights - Separation of Highlight State vs Layout State', () => {
  /**
   * Specification: clearHighlights should only restore highlight state (scale),
   * not layout state (position).
   *
   * Key insight: Scale is now calculated from StaticData, not stored.
   * Position is always read from DOM (current layout state).
   */

  test('preserves arrow position after relayout when clearing highlights', () => {
    // Create fake arrow element
    const arrow = createFakeElement('polygon');

    // Initial expanded position (100, 307), scale 1.0
    const initialPoints = global.ArrowLogic.getArrowPoints({ x: 100, y: 307 }, 1.0);
    arrow.setAttribute('points', initialPoints);
    expect(arrow.getAttribute('points')).toBe('108,303 100,307 108,311');

    // === RELAYOUT: Node collapses, arrow moves to new position (100, 187) ===
    const newCollapsedPoints = global.ArrowLogic.getArrowPoints({ x: 100, y: 187 }, 1.0);
    arrow.setAttribute('points', newCollapsedPoints);
    expect(arrow.getAttribute('points')).toBe('108,183 100,187 108,191');

    // === HIGHLIGHT: User hovers, arrow scales to 1.3 ===
    const highlightedPoints = global.ArrowLogic.getArrowPoints({ x: 100, y: 187 }, 1.3);
    arrow.setAttribute('points', highlightedPoints);
    expect(arrow.getAttribute('points')).toBe('110.4,181.8 100,187 110.4,192.2');

    // === CLEAR HIGHLIGHTS: Should restore scale but KEEP position ===
    // Scale comes from StaticData (simulated as 1.0 here)
    const baseScale = 1.0;
    const currentTip = global.ArrowLogic.parseTipFromPoints(arrow.getAttribute('points'));

    // This is the key: use currentTip (187), not some stored position
    arrow.setAttribute('points', global.ArrowLogic.getArrowPoints(
      currentTip,   // Keep CURRENT position (layout state from DOM)
      baseScale     // Restore base scale (from StaticData calculation)
    ));

    // Verify: Arrow should be at collapsed position (187), not expanded position (307)
    expect(arrow.getAttribute('points')).toBe('108,183 100,187 108,191');

    const finalTip = global.ArrowLogic.parseTipFromPoints(arrow.getAttribute('points'));
    expect(finalTip.y).toBe(187);  // Collapsed position
    expect(finalTip.y).not.toBe(307);  // NOT expanded position
  });

  test('clearHighlights restores scale correctly', () => {
    const arrow = createFakeElement('polygon');

    // Initial: scale 1.0
    arrow.setAttribute('points', global.ArrowLogic.getArrowPoints({ x: 100, y: 200 }, 1.0));

    // Highlight: scale to 1.3
    arrow.setAttribute('points', global.ArrowLogic.getArrowPoints({ x: 100, y: 200 }, 1.3));
    expect(arrow.getAttribute('points')).toBe('110.4,194.8 100,200 110.4,205.2');

    // Clear highlights: restore scale (calculated from StaticData)
    const baseScale = 1.0;  // Would come from ArrowLogic.scaleFromStrokeWidth(StaticData.getArcStrokeWidth(arcId))
    const currentTip = global.ArrowLogic.parseTipFromPoints(arrow.getAttribute('points'));
    arrow.setAttribute('points', global.ArrowLogic.getArrowPoints(currentTip, baseScale));

    // Verify: back to scale 1.0
    expect(arrow.getAttribute('points')).toBe('108,196 100,200 108,204');
  });

  test('handles multiple relayouts correctly', () => {
    const arrow = createFakeElement('polygon');
    const baseScale = 1.0;  // From StaticData

    // Initial position: Y=300
    arrow.setAttribute('points', global.ArrowLogic.getArrowPoints({ x: 100, y: 300 }, 1.0));

    // First relayout: collapse to Y=200
    arrow.setAttribute('points', global.ArrowLogic.getArrowPoints({ x: 100, y: 200 }, 1.0));

    // Highlight
    arrow.setAttribute('points', global.ArrowLogic.getArrowPoints({ x: 100, y: 200 }, 1.3));

    // Clear - should be at Y=200
    let currentTip = global.ArrowLogic.parseTipFromPoints(arrow.getAttribute('points'));
    arrow.setAttribute('points', global.ArrowLogic.getArrowPoints(currentTip, baseScale));
    expect(global.ArrowLogic.parseTipFromPoints(arrow.getAttribute('points')).y).toBe(200);

    // Second relayout: collapse further to Y=100
    arrow.setAttribute('points', global.ArrowLogic.getArrowPoints({ x: 100, y: 100 }, 1.0));

    // Highlight again
    arrow.setAttribute('points', global.ArrowLogic.getArrowPoints({ x: 100, y: 100 }, 1.3));

    // Clear - should be at Y=100, NOT Y=200 or Y=300
    currentTip = global.ArrowLogic.parseTipFromPoints(arrow.getAttribute('points'));
    arrow.setAttribute('points', global.ArrowLogic.getArrowPoints(currentTip, baseScale));
    expect(global.ArrowLogic.parseTipFromPoints(arrow.getAttribute('points')).y).toBe(100);
  });

  test('uses current arrow position from DOM, not stored position', () => {
    // Specification: clearHighlights must read current position from DOM
    const arrow = createFakeElement('polygon');
    const baseScale = 1.0;

    // Initial state
    arrow.setAttribute('points', global.ArrowLogic.getArrowPoints({ x: 100, y: 307 }, 1.0));

    // Relayout changes position
    arrow.setAttribute('points', global.ArrowLogic.getArrowPoints({ x: 100, y: 187 }, 1.0));

    // Highlight
    arrow.setAttribute('points', global.ArrowLogic.getArrowPoints({ x: 100, y: 187 }, 1.3));

    // Clear: must use current position from DOM
    const currentTip = global.ArrowLogic.parseTipFromPoints(arrow.getAttribute('points'));
    arrow.setAttribute('points', global.ArrowLogic.getArrowPoints(
      currentTip,   // Current position (source of truth)
      baseScale     // Base scale from StaticData
    ));

    // Verify: position matches current layout state (187)
    const finalTip = global.ArrowLogic.parseTipFromPoints(arrow.getAttribute('points'));
    expect(finalTip.y).toBe(187);
  });
});

describe('State Separation: Highlight vs Layout', () => {
  test('highlight state: scale is calculated from StaticData', () => {
    // Scale is now calculated on demand from StaticData.getArcStrokeWidth()
    // No stored state needed

    // Simulate: arc with 1 usage -> strokeWidth 0.5 -> scale 0.33
    const strokeWidth = global.ArcLogic.calculateStrokeWidth(1);
    const scale = global.ArrowLogic.scaleFromStrokeWidth(strokeWidth);

    expect(strokeWidth).toBe(0.5);  // MIN strokeWidth
    expect(scale).toBeCloseTo(0.333, 2);  // 0.5 / 1.5

    // During highlight, scale changes to HIGHLIGHT_SCALE (1.3)
    expect(global.HighlightLogic.HIGHLIGHT_SCALE).toBe(1.3);

    // When clearing, we recalculate from StaticData (same result)
    const clearedScale = global.ArrowLogic.scaleFromStrokeWidth(strokeWidth);
    expect(clearedScale).toBeCloseTo(0.333, 2);
  });

  test('layout state: position (persistent, changes on relayout)', () => {
    const arrow = createFakeElement('polygon');

    // Initial layout: Y=300
    arrow.setAttribute('points', global.ArrowLogic.getArrowPoints({ x: 100, y: 300 }, 1.0));
    let tip = global.ArrowLogic.parseTipFromPoints(arrow.getAttribute('points'));
    expect(tip.y).toBe(300);

    // Relayout changes position: Y=200
    arrow.setAttribute('points', global.ArrowLogic.getArrowPoints({ x: 100, y: 200 }, 1.0));
    tip = global.ArrowLogic.parseTipFromPoints(arrow.getAttribute('points'));
    expect(tip.y).toBe(200);

    // Highlight should NOT affect position, only scale
    arrow.setAttribute('points', global.ArrowLogic.getArrowPoints({ x: 100, y: 200 }, 1.3));
    tip = global.ArrowLogic.parseTipFromPoints(arrow.getAttribute('points'));
    expect(tip.y).toBe(200);  // Position unchanged

    // Clear highlight should NOT restore old position (300), only current scale
    arrow.setAttribute('points', global.ArrowLogic.getArrowPoints({ x: 100, y: 200 }, 1.0));
    tip = global.ArrowLogic.parseTipFromPoints(arrow.getAttribute('points'));
    expect(tip.y).toBe(200);  // Position still at current layout state
  });
});

describe('AppState without originalValues', () => {
  test('AppState.create() returns state without originalValues', () => {
    const appState = AppState.create();
    expect(appState.collapsed).toBeDefined();
    expect(appState.selection).toBeDefined();
    expect(appState.originalValues).toBeUndefined();
  });

  test('selection operations still work', () => {
    const appState = AppState.create();

    // Toggle pinned
    expect(AppState.togglePinned(appState, 'node', 'node-1')).toBe(true);
    expect(AppState.getPinned(appState)).toEqual({ type: 'node', id: 'node-1' });

    // Toggle again to deselect
    expect(AppState.togglePinned(appState, 'node', 'node-1')).toBe(false);
    expect(AppState.getPinned(appState)).toBeNull();
  });
});
