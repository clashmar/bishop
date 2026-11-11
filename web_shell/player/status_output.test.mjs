import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { runInNewContext } from 'node:vm';

const playerHtml = new URL('../player.html', import.meta.url);

async function statusOutput() {
  const html = await readFile(playerHtml, 'utf8');
  const inlineScript = html.match(/<script>([\s\S]*?)<\/script>/)?.[1];
  assert.ok(inlineScript, 'player page needs its status-output script');
  const status = {
    textContent: 'Waiting to play',
    scrollTop: 0,
    get scrollHeight() { return this.textContent.length; },
  };
  const play = { hidden: false };
  const frames = [];
  const printed = [];
  const errors = [];
  const window = {};
  runInNewContext(inlineScript, {
    document: { getElementById: id => ({ status, play })[id] },
    window,
    console: { log: message => printed.push(message), error: message => errors.push(message) },
    requestAnimationFrame: callback => frames.push(callback),
  });
  return { status, play, frames, printed, errors, window };
}

test('status output batches bursts and retains only recent messages', async () => {
  const { status, frames, printed, window } = await statusOutput();
  const messages = Array.from({ length: 1000 }, (_, index) => `sensor stay ${index}`);

  for (const message of messages) window.bishopAppendStatus(message);

  assert.equal(status.textContent, 'Waiting to play', 'DOM should update once per frame');
  assert.equal(frames.length, 1, 'a burst should schedule one DOM update');
  assert.equal(printed.length, messages.length, 'console should retain all diagnostics');

  frames.shift()();

  assert.ok(status.textContent.includes(messages.at(-1)), 'latest output should remain visible');
  assert.ok(!status.textContent.includes(`${messages[0]}\n`), 'old output should be discarded');
  assert.ok(status.textContent.length < messages.length * messages[0].length / 2);
  assert.equal(status.scrollTop, status.scrollHeight);
});

test('player ready hides Play before the next status repaint', async () => {
  const { status, play, frames, window } = await statusOutput();

  window.bishopAppendStatus('BISHOP_PLAYER_READY');

  assert.equal(play.hidden, true);
  assert.equal(status.textContent, 'Waiting to play');
  frames.shift()();
  assert.match(status.textContent, /BISHOP_PLAYER_READY/);
});

test('failure output remains visible and disables the loading button', async () => {
  const { status, play, frames, errors, window } = await statusOutput();

  window.Module.printErr('BISHOP_PLAYER_ERROR unable to load game');

  assert.equal(play.textContent, 'Loading failed');
  assert.equal(errors.length, 1);
  frames.shift()();
  assert.match(status.textContent, /ERROR: BISHOP_PLAYER_ERROR unable to load game/);
});

test('one oversized message cannot leave an unbounded status display', async () => {
  const { status, frames, window } = await statusOutput();
  const oversized = 'x'.repeat(100_000);

  window.bishopAppendStatus(oversized);
  frames.shift()();

  assert.ok(status.textContent.length < oversized.length / 2);
  window.bishopAppendStatus('following message');
  frames.shift()();
  assert.ok(status.textContent.endsWith('following message'));
});
