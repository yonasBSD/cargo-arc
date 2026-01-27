// arrow_logic.js - Pure geometry functions for arrow rendering
// No DOM dependencies

// Arrow base dimensions (at scale 1.0)
const ARROW_LENGTH = 8;
const ARROW_HALF_WIDTH = 4;

const ArrowLogic = {
  /**
   * Generate scaled arrow polygon points string.
   * Arrow tip is at tip position, pointing left.
   * @param {{x: number, y: number}} tip - Arrow tip coordinates
   * @param {number} scale - Scale factor (1.0 = original size)
   * @returns {string} - SVG polygon points string
   */
  getArrowPoints(tip, scale) {
    const len = ARROW_LENGTH * scale;
    const hw = ARROW_HALF_WIDTH * scale;
    return `${tip.x + len},${tip.y - hw} ${tip.x},${tip.y} ${tip.x + len},${tip.y + hw}`;
  },

  /**
   * Parse arrow tip position from SVG points string.
   * @param {string} points - SVG polygon points (format: "x1,y1 tipX,tipY x2,y2")
   * @returns {{x: number, y: number}|null}
   */
  parseTipFromPoints(points) {
    const parts = points.split(' ');
    if (parts.length >= 2) {
      const [x, y] = parts[1].split(',').map(Number);
      if (isNaN(x) || isNaN(y)) return null;
      return { x, y };
    }
    return null;
  },

  /**
   * Calculate scale factor from stroke width.
   * @param {number} strokeWidth
   * @returns {number}
   */
  scaleFromStrokeWidth(strokeWidth) {
    return strokeWidth / 1.5; // 1.5 was the original base stroke-width
  },

  // Expose constants for testing
  ARROW_LENGTH,
  ARROW_HALF_WIDTH
};

// CommonJS export for tests (Node/Bun)
if (typeof module !== 'undefined') {
  module.exports = { ArrowLogic };
}
