export function resolveGameUrl(raw, playerHref) {
  if (typeof raw !== 'string' || !raw.trim()) {
    throw new Error('A Bishop game URL is required');
  }
  const player = new URL(playerHref);
  if (player.protocol !== 'http:' && player.protocol !== 'https:') {
    throw new Error('Bishop Player requires HTTP(S) hosting');
  }
  const game = new URL(raw, player);
  if (game.origin !== player.origin) {
    throw new Error('The game URL must use the same origin as Bishop Player');
  }
  if (game.search || game.hash) {
    throw new Error('The game URL must not include a query or fragment');
  }
  if (game.pathname.endsWith('/')) {
    throw new Error('The game URL must name a manifest file');
  }
  return game;
}
