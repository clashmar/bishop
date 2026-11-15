import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { runInNewContext } from 'node:vm';

const audioScript = new URL('../audio.js', import.meta.url);

async function audioBridge() {
  const canvas = {};
  const contexts = [];
  const pointers = [];
  let tick;
  class AudioContext {
    constructor({ sampleRate }) {
      this.sampleRate = sampleRate;
      this.state = 'suspended';
      this.currentTime = 0;
      contexts.push(this);
    }
    resume() { this.state = 'running'; return Promise.resolve(); }
    close() { this.state = 'closed'; return Promise.resolve(); }
    createBuffer(_channels, frames) {
      return { getChannelData: () => new Float32Array(frames) };
    }
    createBufferSource() {
      return { connect() {}, disconnect() {}, start() {}, stop() {} };
    }
  }
  const window = {
    AudioContext,
    bishopAppendStatus() {},
    Module: {
      ccall(_name, _type, _types, args) { pointers.push(args[0]); return 4; },
      HEAPF32: new Float32Array(64),
    },
  };
  const source = await readFile(audioScript, 'utf8');
  runInNewContext(source, {
    window,
    document: { getElementById: () => canvas, activeElement: canvas, hidden: false },
    setInterval: callback => { tick = callback; return 1; },
    clearInterval() {},
  });
  return { window, contexts, pointers, tick: () => tick() };
}

test('replacing an engine audio callback keeps the context alive until both backends stop', async () => {
  const { window, contexts, pointers, tick } = await audioBridge();
  assert.equal(window.bishopAudioStart(100, 48000, 4), 1);
  await Promise.resolve();
  assert.equal(pointers.at(-1), 100);

  assert.equal(window.bishopAudioStart(200, 48000, 4), 1);
  window.bishopAudioStop(100);
  assert.equal(window.bishopAudioResume(), 1);
  contexts[0].currentTime = 1;
  tick();

  assert.equal(pointers.at(-1), 200);
  assert.equal(contexts.length, 1);
  assert.equal(contexts[0].state, 'running');
  window.bishopAudioStop(200);
  assert.equal(contexts[0].state, 'closed');
});

test('failed replacement restores the previous engine audio callback', async () => {
  const { window, contexts, pointers, tick } = await audioBridge();
  assert.equal(window.bishopAudioStart(100, 48000, 4), 1);
  await Promise.resolve();
  assert.equal(window.bishopAudioStart(200, 48000, 4), 1);

  window.bishopAudioStop(200);
  contexts[0].currentTime = 1;
  tick();

  assert.equal(pointers.at(-1), 100);
  assert.equal(window.bishopAudioResume(), 1);
  window.bishopAudioStop(100);
  assert.equal(contexts[0].state, 'closed');
});
