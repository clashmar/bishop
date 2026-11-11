import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { runInNewContext } from 'node:vm';

const playerHtml = new URL('../player.html', import.meta.url);
const embedExampleHtml = new URL('../embed-example.html', import.meta.url);

test('player defaults to a full-width 1024×576 canvas', async () => {
  const html = await readFile(playerHtml, 'utf8');
  assert.match(html, /main\s*\{[^}]*max-width:\s*1024px;[^}]*padding:\s*1rem 0;/);
  assert.match(html, /<canvas id="bishop-canvas" width="1024" height="576"/);
});

test('example iframe leaves room for the canvas and player controls', async () => {
  const html = await readFile(embedExampleHtml, 'utf8');
  assert.match(html, /iframe\s*\{[^}]*height:\s*850px;/);
});

async function installCanvasSizing(canvas) {
  const html = await readFile(playerHtml, 'utf8');
  const scripts = [...html.matchAll(/<script(?:\s[^>]*)?>([\s\S]*?)<\/script>/g)];
  const source = scripts.map(([, body]) => body).find(body => body.includes('ResizeObserver'));
  assert.ok(source, 'browser player needs responsive backing-store sizing');
  let resize;
  runInNewContext(source, {
    document: { getElementById: id => id === 'bishop-canvas' ? canvas : null },
    ResizeObserver: class {
      constructor(callback) { resize = callback; }
      observe(target) { assert.equal(target, canvas); }
    },
  });
  return () => resize();
}

test('browser canvas backing follows its displayed size without resetting unchanged buffers', async () => {
  let displayedWidth = 800;
  let backingWidth = 960;
  let backingHeight = 540;
  let widthWrites = 0;
  let heightWrites = 0;
  const canvas = {
    get width() { return backingWidth; },
    set width(value) { backingWidth = value; widthWrites++; },
    get height() { return backingHeight; },
    set height(value) { backingHeight = value; heightWrites++; },
    getBoundingClientRect() {
      return { width: displayedWidth, height: displayedWidth * backingHeight / backingWidth };
    },
  };
  const resize = await installCanvasSizing(canvas);

  resize();
  assert.deepEqual([canvas.width, canvas.height], [800, 450]);
  resize();
  assert.deepEqual([widthWrites, heightWrites], [1, 1]);

  displayedWidth = 960;
  resize();
  assert.deepEqual([canvas.width, canvas.height], [960, 540]);
});

test('browser canvas preserves its backing size while hidden', async () => {
  const canvas = {
    width: 960,
    height: 540,
    getBoundingClientRect: () => ({ width: 0, height: 0 }),
  };
  const resize = await installCanvasSizing(canvas);

  resize();

  assert.deepEqual([canvas.width, canvas.height], [960, 540]);
});
