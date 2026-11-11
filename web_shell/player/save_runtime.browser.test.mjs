import assert from 'node:assert/strict';
import { execFile, spawn } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { existsSync } from 'node:fs';
import { cp, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';
import { chromeBinary, connectCdp, serveStaticSite, until } from './browser_test_support.mjs';

const root = resolve(fileURLToPath(new URL('../../', import.meta.url)));
const fixtureRoot = join(root, 'xtask/tests/fixtures/web_smoke/Resources');
const fixtureName = 'BrowserSaveFixture';
const run = promisify(execFile);

async function authorFixture(resources) {
  await cp(fixtureRoot, resources, { recursive: true });
  const gamePath = join(resources, 'game.ron');
  let game = await readFile(gamePath, 'utf8');
  assert.match(game, /id: "[0-9a-f-]+"/);
  game = game.replace(/id: "[0-9a-f-]+"/, `id: "${randomUUID()}"`)
    .replace(/name: "[^"]+"/, `name: "${fixtureName}"`);
  await writeFile(gamePath, game);
  const gameId = game.match(/id: "([0-9a-f-]+)"/)[1];
  const gameName = game.match(/name: "([^"]+)"/)[1];
  const startWorldId = game.match(/start_world_id: Some\(\((\d+)\)\)/)?.[1];
  assert.ok(startWorldId, 'authored fixture needs a start world');
  const worldPath = join(resources, `worlds/world-${startWorldId}.ron`);
  let world = await readFile(worldPath, 'utf8');
  const startRoomId = Number(world.match(/current_room_id: Some\(\((\d+)\)\)/)?.[1]);
  const roomSize = world.match(/size: \(([\d.]+), ([\d.]+)\)/);
  const gridSize = Number(world.match(/grid_size: ([\d.]+)/)?.[1]);
  assert.ok(startRoomId && roomSize && gridSize, 'authored fixture needs a sized start room');
  const savedRoomId = startRoomId + 1;
  const rightEdge = Number(roomSize[1]) * gridSize;
  const exitCellY = 3;
  const entryX = rightEdge + gridSize / 2;
  const entryY = (exitCellY + 1) * gridSize;
  assert.ok(world.includes('exits: [],\n            adjacent_rooms: []'));
  world = world.replace('exits: [],\n            adjacent_rooms: []',
    `exits: [(position: (${(Number(roomSize[1]) - 1).toFixed(1)}, ${exitCellY.toFixed(1)}), direction: Right, layer: front, target_room_id: Some((${savedRoomId})))],\n            adjacent_rooms: [(${savedRoomId})]`);
  const roomEnd = world.match(/            singleton: \((\d+)\),\n        \),/);
  assert.ok(roomEnd, 'authored start room needs a singleton');
  const [firstRoomEnd, singletonId] = roomEnd;
  world = world.replace(firstRoomEnd,
    `${firstRoomEnd}\n        (id: (${savedRoomId}), name: "Saved Room", position: (${rightEdge.toFixed(1)}, 0.0), size: (${roomSize[1]}, ${roomSize[2]}), exits: [], adjacent_rooms: [(${startRoomId})], tags: [Autosave], singleton: (${singletonId})),`);
  await writeFile(worldPath, world);
  assert.equal(Number(world.match(/id: \((\d+)\), name: "Saved Room"/)[1]), savedRoomId);
  const roomPayload = (await readFile(join(resources, `payloads/room-${startRoomId}.ron`), 'utf8'))
    .replace(/entities: \[[\s\S]*?\],/, 'entities: [],');
  await writeFile(join(resources, `payloads/room-${savedRoomId}.ron`), roomPayload);
  const playerScriptId = 42;
  const registryPath = join(resources, 'asset_registry.ron');
  const registry = await readFile(registryPath, 'utf8');
  await writeFile(registryPath, registry.replace('    records: {',
    `    records: {\n        Script((${playerScriptId})): (path: "scripts/player.lua"),`));
  const ecsPath = join(resources, 'ecs.ron');
  const ecs = await readFile(ecsPath, 'utf8');
  const playerEntity = ecs.match(/type_name: "Player",\s*data: \{\s*\((\d+)\)/)?.[1];
  assert.ok(playerEntity, 'authored fixture needs a player');
  await writeFile(ecsPath, ecs.replace('        (\n            type_name: "Sprite",',
    `        (\n            type_name: "Script",\n            data: { (${playerEntity}): (script_id: (${playerScriptId}), data: (fields: {})), },\n        ),\n        (\n            type_name: "Sprite",`));
  await writeFile(join(resources, 'scripts/player.lua'), 'return { init = function(self) end }\n');
  const titlePath = join(resources, 'menus/title.ron');
  let title = await readFile(titlePath, 'utf8');
  const newGame = title.match(/(\/\*\[1\]\*\/ \([\s\S]*?\n\s*managed: true,\n\s*\),)/)?.[1];
  assert.ok(newGame, 'authored title must contain an Enter Workshop button');
  const loadGame = newGame.replace('/*[1]*/', '/*[2]*/')
    .replace('name: "EnterWorkshop"', 'name: "LoadGame"')
    .replace('text_key: "EnterWorkshop"', 'text_key: "LoadGame"')
    .replace('action: CloseMenu', 'action: Custom("load_latest")')
    .replace('enabled: true,', 'enabled: true,\n                            enabled_when: HasLatestSave,')
    .replace('y: 0.1310237', 'y: 0.2310237');
  title = title.replace(newGame, `${newGame}\n                    ${loadGame}`);
  await writeFile(titlePath, title);
  await writeFile(join(resources, 'text/en/ui/title.toml'),
    `${await readFile(join(resources, 'text/en/ui/title.toml'), 'utf8')}LoadGame = "Load Latest"\n`);
  await writeFile(join(resources, 'scripts/main.lua'), `
engine.save.register_provider({
    id = "fixture.room", version = 1,
    capture = function()
        return tostring(engine.player().entity:current_room())
    end,
    apply = function(data)
        local room = tonumber(data)
        engine.log.info("SAVE_FIXTURE_RESTORE_APPLY " .. room)
        engine.restore_location({world_id = engine.current_world().id, room_id = room, x = ${entryX}, y = ${entryY}})
    end,
})
engine.log.info("SAVE_FIXTURE_HAS_LATEST " .. tostring(engine.save.has_latest()))
engine.on("menu:load_latest", function()
    engine.log.info("SAVE_FIXTURE_LOAD_REQUESTED")
    engine.save.load_latest()
end)
engine.on(engine.events.room_entered, function(room_id, ...)
    for _, tag in ipairs({...}) do
        if tag == engine.tags.Autosave then
            engine.log.info("SAVE_FIXTURE_ROOM_ENTERED " .. room_id)
            engine.save.auto()
        end
    end
end)
engine.on(engine.events.save_succeeded, function(lane, trigger)
    engine.log.info("SAVE_FIXTURE_SUCCEEDED " .. lane .. " " .. trigger)
end)
engine.on(engine.events.save_failed, function(lane, trigger, error)
    engine.log.info("SAVE_FIXTURE_FAILED " .. lane .. " " .. trigger .. " " .. error)
end)
local lastRoom
engine.update = function(_dt)
    local player = engine.player()
    if not player then return end
    local room = player.entity:current_room()
    if room ~= lastRoom then
        lastRoom = room
        engine.log.info("SAVE_FIXTURE_ROOM " .. room)
    end
    if room == ${startRoomId} and engine.input.pressed("t") then
        player.entity:teleport({x = ${entryX}, y = ${entryY}})
    end
end
`);
  return { gameId, gameName, startRoomId, savedRoomId };
}

async function clickCanvas(cdp, x, y) {
  const rect = await cdp.evaluate('document.getElementById("bishop-canvas").getBoundingClientRect().toJSON()');
  const px = rect.left + rect.width * x, py = rect.top + rect.height * y;
  await cdp.command('Input.dispatchMouseEvent', { type: 'mouseMoved', x: px, y: py });
  await cdp.command('Input.dispatchMouseEvent', { type: 'mousePressed', button: 'left', x: px, y: py, clickCount: 1 });
  await new Promise(resolve => setTimeout(resolve, 120));
  await cdp.command('Input.dispatchMouseEvent', { type: 'mouseReleased', button: 'left', x: px, y: py, clickCount: 1 });
}

async function pressT(cdp) {
  for (const type of ['rawKeyDown', 'keyUp']) {
    await cdp.command('Input.dispatchKeyEvent', {
      type, key: 't', code: 'KeyT', windowsVirtualKeyCode: 84,
    });
    if (type === 'rawKeyDown') await new Promise(resolve => setTimeout(resolve, 120));
  }
}

async function status(cdp) {
  const text = await cdp.evaluate('document.getElementById("status")?.textContent || ""');
  assert.doesNotMatch(text, /Could not resolve app directory|Aborted\(\)|BISHOP_PLAYER_ERROR/);
  return text;
}

async function waitForPlayPage(cdp) {
  await until(async () => {
    try { return await cdp.evaluate('document.getElementById("status")?.textContent.trim() === "Waiting to play"'); }
    catch { return false; }
  }, 'player page');
}

async function play(cdp) {
  await waitForPlayPage(cdp);
  await until(async () => cdp.evaluate('Boolean(window.bishopSaveStore)'), 'player module ready', 15000);
  await cdp.evaluate('document.getElementById("play").click()');
  await until(async () => (await status(cdp)).includes('BISHOP_PLAYER_READY'), 'player ready', 60000);
  assert.equal(
    await cdp.evaluate('document.getElementById("bishop-canvas").getContext("webgl2").getContextAttributes().antialias'),
    false,
    'pixel-art player canvas must not multisample sprite edges',
  );
}

async function sampleGameplayEdges(cdp) {
  await cdp.evaluate('new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)))');
  const screenshot = await cdp.command('Page.captureScreenshot', { format: 'png' });
  return cdp.evaluate(`(async () => {
    const image = new Image();
    image.src = ${JSON.stringify(`data:image/png;base64,${screenshot.data}`)};
    await image.decode();
    const scratch = document.createElement('canvas');
    scratch.width = image.width; scratch.height = image.height;
    const context = scratch.getContext('2d');
    context.drawImage(image, 0, 0);
    const rect = document.getElementById('bishop-canvas').getBoundingClientRect();
    const y = Math.floor(rect.top + rect.height * 0.75);
    const pixel = x => Array.from(context.getImageData(Math.floor(x), y, 1, 1).data).slice(0, 3);
    return { right: pixel(rect.right - 12), inside: pixel(rect.left + rect.width * 0.75) };
  })()`);
}

test('browser room-exit autosave restores after reload via Load Latest', async t => {
  assert.ok(existsSync(chromeBinary), `Chrome not found at ${chromeBinary}`);
  const temporary = await mkdtemp(join(tmpdir(), 'bishop-save-runtime-'));
  const profile = join(temporary, 'chrome');
  const resources = join(temporary, 'Resources');
  const site = join(temporary, 'site');
  const server = serveStaticSite(site);
  let chrome, cdp;
  t.after(async () => {
    cdp?.socket.close();
    if (chrome && chrome.exitCode === null) {
      await new Promise(resolve => {
        const timer = setTimeout(resolve, 5000);
        chrome.once('exit', () => { clearTimeout(timer); resolve(); });
        chrome.kill();
      });
    }
    await new Promise(resolve => server.close(resolve));
    await rm(temporary, { recursive: true, force: true, maxRetries: 8, retryDelay: 100 });
  });
  const { gameId, gameName, startRoomId, savedRoomId } = await authorFixture(resources);
  await run('cargo', ['xtask', 'export-web-game', '--resources', resources, '--out', site], { cwd: root });
  assert.match(await readFile(join(site, 'games', gameName, 'game.ron'), 'utf8'), new RegExp(gameId));
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const url = `http://127.0.0.1:${server.address().port}/bishop-player/player.html?game=../games/${gameName}/manifest.ron`;
  chrome = spawn(chromeBinary, [
    '--headless=new', '--no-first-run', '--disable-extensions', '--no-default-browser-check',
    '--remote-allow-origins=*', '--remote-debugging-port=0', `--user-data-dir=${profile}`,
    '--window-size=1100,850', url,
  ], { stdio: 'ignore' });
  cdp = await connectCdp(profile);
  await cdp.command('Page.enable');
  await cdp.command('Runtime.enable');
  await play(cdp);
  assert.match(await status(cdp), /SAVE_FIXTURE_HAS_LATEST false/);
  await clickCanvas(cdp, 0.5, 0.555);
  assert.doesNotMatch(await status(cdp), /SAVE_FIXTURE_LOAD_REQUESTED/);
  await clickCanvas(cdp, 0.5, 0.502);
  await until(async () => (await status(cdp)).includes(`SAVE_FIXTURE_ROOM ${startRoomId}`), 'start room')
    .catch(async error => { throw new Error(`${error.message}: ${(await status(cdp)).slice(-2500)}`); });
  const canvasSize = await cdp.evaluate(`(() => {
    const canvas = document.getElementById('bishop-canvas');
    const rect = canvas.getBoundingClientRect();
    return { backing: [canvas.width, canvas.height], displayed: [rect.width, rect.height] };
  })()`);
  assert.deepEqual(canvasSize, { backing: [1024, 576], displayed: [1024, 576] });
  const pixels = await sampleGameplayEdges(cdp);
  assert.ok(pixels.right.every(channel => channel > 100), 'gameplay should reach the right canvas edge');
  assert.ok(pixels.inside.every(channel => channel > 100), 'gameplay should render inside the viewport');
  await pressT(cdp);
  await until(async () => (await status(cdp)).includes(`SAVE_FIXTURE_ROOM_ENTERED ${savedRoomId}`), 'room exit');
  await until(async () => (await status(cdp)).includes('SAVE_FIXTURE_SUCCEEDED autosave auto'), 'durable autosave');
  assert.match(await status(cdp), new RegExp(`SAVE_FIXTURE_ROOM ${savedRoomId}`));
  await cdp.command('Page.reload', { ignoreCache: true });
  await play(cdp);
  assert.match(await status(cdp), /SAVE_FIXTURE_HAS_LATEST true/);
  const scrollBeforeClick = await cdp.evaluate('scrollY');
  await clickCanvas(cdp, 0.5, 0.555);
  assert.equal(await cdp.evaluate('scrollY'), scrollBeforeClick, 'focusing the canvas must not scroll the menu');
  await until(async () => (await status(cdp)).includes('SAVE_FIXTURE_LOAD_REQUESTED'), 'custom Load Latest action');
  await until(async () => (await status(cdp)).includes(`SAVE_FIXTURE_RESTORE_APPLY ${savedRoomId}`),
    'Load Latest provider restoration', 10000);
  await until(async () => (await status(cdp)).includes(`SAVE_FIXTURE_ROOM ${savedRoomId}`),
    'restored room', 10000);

  const committed = await cdp.evaluate(`window.bishopSaveStore.open(${JSON.stringify(gameId)})`);
  assert.match(committed.latest, /data: "\d+"/);
  await cdp.command('Page.reload', { ignoreCache: true });
  await play(cdp);
  await cdp.evaluate(`(async () => {
    const { DOCUMENTS_STORE } = await import('./player/save_store.mjs');
    const original = IDBDatabase.prototype.transaction;
    IDBDatabase.prototype.transaction = function(names, mode, ...args) {
      const transaction = original.call(this, names, mode, ...args);
      if (mode === 'readwrite' && names.includes(DOCUMENTS_STORE)) {
        queueMicrotask(() => transaction.abort());
      }
      return transaction;
    };
    return true;
  })()`);
  await clickCanvas(cdp, 0.5, 0.502);
  await until(async () => (await status(cdp)).includes(`SAVE_FIXTURE_ROOM ${startRoomId}`),
    'aborted-write start room');
  await pressT(cdp);
  await until(async () => (await status(cdp)).includes(`SAVE_FIXTURE_ROOM ${savedRoomId}`),
    'aborted-write room traversal');
  await until(async () => (await status(cdp)).includes('SAVE_FIXTURE_FAILED autosave auto'),
    'aborted transaction failure event');
  const afterAbort = await cdp.evaluate(`window.bishopSaveStore.open(${JSON.stringify(gameId)})`);
  assert.equal(afterAbort.latest, committed.latest);

  await cdp.command('Page.reload', { ignoreCache: true });
  await play(cdp);
  assert.match(await status(cdp), /SAVE_FIXTURE_HAS_LATEST true/);
  const corrupt = committed.latest.replace(/data: "\d+"/, 'data: "invalid room"');
  await cdp.evaluate(`window.bishopSaveStore.write(${JSON.stringify(gameId)}, 'default',
    'autosave', ${JSON.stringify(corrupt)}, 999).then(() => true)`);
  await cdp.command('Page.reload', { ignoreCache: true });
  await play(cdp);
  assert.match(await status(cdp), /SAVE_FIXTURE_HAS_LATEST true/);
  await clickCanvas(cdp, 0.5, 0.555);
  await until(async () => (await status(cdp)).includes('SAVE_FIXTURE_LOAD_REQUESTED'), 'corrupt custom load action');
  await until(async () => (await status(cdp)).includes('BISHOP_PLAYER_LOAD_ERROR'), 'corrupt save error');
  await clickCanvas(cdp, 0.5, 0.502);
  await until(async () => (await status(cdp)).includes(`SAVE_FIXTURE_ROOM ${startRoomId}`),
    'old engine remains playable after failed restore');

  await cdp.command('Page.reload', { ignoreCache: true });
  await waitForPlayPage(cdp);
  await cdp.evaluate('Object.defineProperty(window, "indexedDB", { configurable: true, value: undefined }); true');
  await play(cdp);
  assert.match(await status(cdp), /BISHOP_PLAYER_SAVE_UNAVAILABLE/);
  assert.match(await status(cdp), /SAVE_FIXTURE_HAS_LATEST false/);
  await clickCanvas(cdp, 0.5, 0.502);
  await until(async () => (await status(cdp)).includes(`SAVE_FIXTURE_ROOM ${startRoomId}`), 'denied-storage start room');
  await pressT(cdp);
  await until(async () => (await status(cdp)).includes(`SAVE_FIXTURE_ROOM ${savedRoomId}`),
    'denied-storage room traversal');
  await until(async () => (await status(cdp)).includes('SAVE_FIXTURE_FAILED autosave auto'),
    'denied-storage save failure');

  await cdp.command('Page.navigate', { url: new URL('../index.html', url).href });
  await until(async () => cdp.evaluate('Boolean(document.getElementById("reveal"))'), 'exported example page');
  await cdp.evaluate('document.getElementById("reveal").click()');
  await until(async () => cdp.evaluate('Boolean(document.querySelector("iframe")?.contentDocument?.getElementById("bishop-canvas"))'),
    'exported player iframe');
  const exampleSize = await cdp.evaluate(`(() => {
    const iframe = document.querySelector('iframe');
    const canvas = iframe.contentDocument.getElementById('bishop-canvas');
    return { iframe: [iframe.clientWidth, iframe.clientHeight], canvas: [canvas.width, canvas.height] };
  })()`);
  assert.deepEqual(exampleSize, { iframe: [1024, 850], canvas: [1024, 576] });
});
