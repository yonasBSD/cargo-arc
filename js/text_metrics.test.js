import { test, expect, describe } from "bun:test";
import { TextMetrics } from "./text_metrics.js";

describe("TextMetrics", () => {
  describe("estimateWidth", () => {
    test("returns 0 for empty string", () => {
      expect(TextMetrics.estimateWidth("")).toBe(0);
    });

    test("returns 0 for null/undefined", () => {
      expect(TextMetrics.estimateWidth(null)).toBe(0);
      expect(TextMetrics.estimateWidth(undefined)).toBe(0);
    });

    test("calculates width for 11px font (default)", () => {
      // 10 chars * 6.6px = 66px
      expect(TextMetrics.estimateWidth("0123456789")).toBe(66);
    });

    test("calculates width for 10px font", () => {
      // 10 chars * 6.0px = 60px
      expect(TextMetrics.estimateWidth("0123456789", 10)).toBe(60);
    });

    test("calculates width for typical label text", () => {
      // "Symbol  <- file.rs:10" = 21 chars * 6.6 = 138.6
      const text = "Symbol  <- file.rs:10";
      expect(TextMetrics.estimateWidth(text)).toBeCloseTo(138.6, 1);
    });

    test("uses 11px width for unknown font sizes", () => {
      // Unknown font size falls back to 11px calculation
      expect(TextMetrics.estimateWidth("abc", 14)).toBe(3 * 6.6);
    });
  });

});
