import assert from 'node:assert/strict';
import test from 'node:test';
import { resolveGameUrl } from './config.mjs';

const playerUrl = 'https://example.test/site/bishop-player/player.html';

test('player URL resolves a selected game relative to its own page', () => {
  const url = resolveGameUrl('../games/Arcade/manifest.ron', playerUrl);
  assert.equal(url.href, 'https://example.test/site/games/Arcade/manifest.ron');
});

test('player URL rejects a game hosted on another origin', () => {
  assert.throws(() => resolveGameUrl('https://other.test/game/manifest.ron', playerUrl),
                /same origin/);
});

test('player URL rejects missing games and non-HTTP hosting', () => {
  assert.throws(() => resolveGameUrl(null, playerUrl), /game URL/);
  assert.throws(() => resolveGameUrl('games/Arcade/manifest.ron', 'file:///tmp/player.html'),
                /HTTP/);
});

test('player URL rejects manifest fragments and query strings', () => {
  assert.throws(() => resolveGameUrl('/games/Arcade/manifest.ron?version=1', playerUrl),
                /query or fragment/);
  assert.throws(() => resolveGameUrl('/games/Arcade/manifest.ron#extra', playerUrl),
                /query or fragment/);
});
