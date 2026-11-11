import { createReadStream } from 'node:fs';
import { readFile, stat } from 'node:fs/promises';
import { createServer } from 'node:http';
import { resolve, sep } from 'node:path';

export const chromeBinary = process.env.CHROME_BIN ?? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';

export async function until(check, description, timeoutMs = 30000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const value = await check();
    if (value) return value;
    await new Promise(resolve => setTimeout(resolve, 50));
  }
  throw new Error(`Timed out waiting for ${description}`);
}

export function serveStaticSite(root) {
  return createServer(async (request, response) => {
    const path = resolve(root, `.${new URL(request.url, 'http://localhost').pathname}`);
    if (!path.startsWith(root + sep)) return response.writeHead(403).end();
    try {
      if (!(await stat(path)).isFile()) throw new Error('not a file');
      response.setHeader('Content-Type', path.endsWith('.wasm') ? 'application/wasm'
        : /\.(mjs|js)$/.test(path) ? 'text/javascript' : 'text/html');
      createReadStream(path).pipe(response);
    } catch { response.writeHead(404).end(); }
  });
}

export async function connectCdp(profile) {
  const port = await until(async () => {
    try { return Number((await readFile(resolve(profile, 'DevToolsActivePort'), 'utf8')).split('\n')[0]); }
    catch { return null; }
  }, 'Chrome DevTools port', 15000);
  const pages = await until(async () => {
    try { return await (await fetch(`http://127.0.0.1:${port}/json/list`)).json(); }
    catch { return null; }
  }, 'Chrome page');
  const page = pages.find(entry => entry.type === 'page');
  if (!page) throw new Error('No Chrome page to inspect');
  const socket = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    socket.addEventListener('open', resolve, { once: true });
    socket.addEventListener('error', reject, { once: true });
  });
  let nextId = 0;
  const pending = new Map();
  socket.addEventListener('message', event => {
    const message = JSON.parse(event.data);
    const waiter = pending.get(message.id);
    if (!waiter) return;
    pending.delete(message.id);
    clearTimeout(waiter.timer);
    if (message.error) waiter.reject(new Error(JSON.stringify(message.error)));
    else waiter.resolve(message.result);
  });
  function command(method, params = {}) {
    const id = ++nextId;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        pending.delete(id);
        reject(new Error(`Timed out calling ${method}`));
      }, 15000);
      pending.set(id, { resolve, reject, timer });
      socket.send(JSON.stringify({ id, method, params }));
    });
  }
  async function evaluate(expression) {
    const result = await command('Runtime.evaluate', {
      expression, awaitPromise: true, returnByValue: true,
    });
    if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
    return result.result.value;
  }
  function nextEvent(method) {
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        socket.removeEventListener('message', onMessage);
        reject(new Error(`Timed out waiting for ${method}`));
      }, 15000);
      function onMessage(event) {
        if (JSON.parse(event.data).method !== method) return;
        clearTimeout(timer);
        socket.removeEventListener('message', onMessage);
        resolve();
      }
      socket.addEventListener('message', onMessage);
    });
  }
  return { socket, command, evaluate, nextEvent };
}
