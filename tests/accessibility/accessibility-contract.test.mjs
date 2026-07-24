import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import test from "node:test";

const css = readFileSync(
  resolve(import.meta.dirname, "..", "..", "src", "app", "app.css"),
  "utf8",
);

function luminance(hex) {
  const channels = [1, 3, 5]
    .map((offset) => Number.parseInt(hex.slice(offset, offset + 2), 16) / 255)
    .map((channel) =>
      channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4,
    );
  return 0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2];
}

function contrast(first, second) {
  const firstLuminance = luminance(first);
  const secondLuminance = luminance(second);
  return (
    (Math.max(firstLuminance, secondLuminance) + 0.05) /
    (Math.min(firstLuminance, secondLuminance) + 0.05)
  );
}

function themeBlock(selector) {
  const start = css.indexOf(selector);
  assert.notEqual(start, -1, `missing theme selector ${selector}`);
  const open = css.indexOf("{", start);
  let depth = 0;
  for (let index = open; index < css.length; index += 1) {
    if (css[index] === "{") depth += 1;
    if (css[index] === "}") depth -= 1;
    if (depth === 0) return css.slice(open + 1, index);
  }
  throw new Error(`unterminated theme selector ${selector}`);
}

function token(block, name) {
  const match = block.match(new RegExp(`--${name}:\\s*(#[0-9a-fA-F]{6})`));
  assert.ok(match, `missing --${name}`);
  return match[1];
}

test("semantic text and filled actions meet WCAG AA in both themes", () => {
  const light = themeBlock(":root {");
  const dark = themeBlock(':root[data-theme="dark"]');

  for (const [name, block, background] of [
    ["light", light, "#ffffff"],
    ["dark", dark, "#23272d"],
  ]) {
    for (const color of [
      "accent-ink",
      "warm-ink",
      "running-ink",
      "danger-ink",
    ]) {
      assert.ok(
        contrast(token(block, color), background) >= 4.5,
        `${name} --${color} does not meet 4.5:1`,
      );
    }
    assert.ok(
      contrast(token(block, "action-ink"), token(block, "accent-action")) >=
        4.5,
      `${name} primary action does not meet 4.5:1`,
    );
    assert.ok(
      contrast(token(block, "action-ink"), token(block, "danger-action")) >=
        4.5,
      `${name} destructive action does not meet 4.5:1`,
    );
  }
});

test("focus and reduced-motion behavior remain explicit", () => {
  assert.match(css, /button:focus-visible\s*\{/);
  assert.match(css, /--focus-shadow:/);
  assert.match(css, /@media \(prefers-reduced-motion: reduce\)/);
  assert.match(css, /:root\[data-reduce-motion="true"\]/);
});

test("semantic colors used for text select their accessible ink variants", () => {
  assert.doesNotMatch(
    css,
    /^\s*color:\s*var\(--(?:accent|warm|running|danger)\)\s*[;!]/m,
  );
});
