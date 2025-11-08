const { test, expect } = require('@playwright/test');
const http = require('http');
const { AddressInfo } = require('net');

async function createStubServer() {
  const events = [];
  const server = http.createServer(async (req, res) => {
    if (req.method === 'GET' && req.url === '/') {
      res.writeHead(200, { 'Content-Type': 'text/html' });
      res.end(renderHtml());
      return;
    }

    if (req.method === 'POST') {
      const body = await readBody(req);
      const payload = body ? JSON.parse(body) : {};

      if (req.url === '/tokens/generate') {
        return json(res, { token: 'stub-token', expires_in: 3600 });
      }
      if (req.url === '/conversations') {
        const conversationId = 'conv-demo-001';
        return json(res, {
          conversationId,
          expires_in: 1800,
          streamUrl: 'wss://stub.greentic.ai/conversations/conv-demo-001',
        });
      }
      if (req.url && req.url.startsWith('/conversations/') && req.url.endsWith('/activities')) {
        events.push({ path: req.url, payload });
        return json(res, { id: 'activity-xyz', accepted: true }, 202);
      }
    }

    res.writeHead(404, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ error: 'not-found' }));
  });

  await new Promise((resolve, reject) => {
    const onError = (err) => reject(err);
    server.once('error', onError);
    server.listen(0, '127.0.0.1', () => {
      server.off('error', onError);
      resolve();
    });
  });
  const { port } = /** @type {AddressInfo} */ (server.address());
  return {
    baseUrl: `http://127.0.0.1:${port}`,
    events,
    async close() {
      await new Promise((resolve, reject) => server.close((err) => (err ? reject(err) : resolve())));
    },
  };
}

function renderHtml() {
  return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8" />
<title>Greentic WebChat Stub</title>
<style>
body { font-family: Arial, sans-serif; margin: 2rem; }
#status { margin-bottom: 1rem; font-weight: 600; }
#menu button { margin-right: 0.5rem; }
#transcript { list-style: none; padding-left: 0; min-height: 5rem; }
#transcript li { margin: 0.25rem 0; }
#last-choice { font-weight: bold; }
</style>
</head>
<body>
  <div id="status">Connecting…</div>
  <div id="menu">
    <button data-choice="about" type="button">About</button>
    <button data-choice="contact" type="button">Contact</button>
  </div>
  <p>Last choice: <span id="last-choice">(none)</span></p>
  <ul id="transcript"></ul>
  <form id="chat-form">
    <input id="chat-input" placeholder="Say something" />
    <button id="chat-send" type="submit">Send</button>
  </form>
<script>
const transcript = document.getElementById('transcript');
const statusEl = document.getElementById('status');
const lastChoice = document.getElementById('last-choice');
let conversationId = null;

function append(role, text) {
  const item = document.createElement('li');
  item.textContent = role + ': ' + text;
  transcript.appendChild(item);
}

async function bootstrap() {
  await fetch('/tokens/generate', { method: 'POST' });
  const convResp = await fetch('/conversations', { method: 'POST' });
  const convData = await convResp.json();
  conversationId = convData.conversationId;
  statusEl.textContent = 'Connected to ' + conversationId;
}

async function postActivity(payload) {
  if (!conversationId) throw new Error('conversation missing');
  await fetch('/conversations/' + conversationId + '/activities', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload)
  });
}

document.querySelectorAll('#menu button').forEach((btn) => {
  btn.addEventListener('click', async () => {
    const choice = btn.dataset.choice;
    lastChoice.textContent = choice;
    append('User', 'Selected ' + choice);
    append('Bot', 'Responding to ' + choice);
    await postActivity({ type: 'choice', choice });
  });
});

document.getElementById('chat-form').addEventListener('submit', async (event) => {
  event.preventDefault();
  const input = document.getElementById('chat-input');
  const text = input.value.trim();
  if (!text) return;
  append('User', text);
  append('Bot', 'echo: ' + text);
  input.value = '';
  await postActivity({ type: 'message', text });
});

bootstrap();
</script>
</body>
</html>`;
}

function readBody(req) {
  return new Promise((resolve) => {
    const chunks = [];
    req.on('data', (chunk) => chunks.push(chunk));
    req.on('end', () => resolve(Buffer.concat(chunks).toString()));
  });
}

function json(res, payload, status = 200) {
  res.writeHead(status, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify(payload));
}

let server;
let skipSuite = false;
let skipReason = '';

test.beforeAll(async () => {
  try {
    server = await createStubServer();
  } catch (error) {
    skipSuite = true;
    skipReason = `Unable to bind stub server (${error.message})`;
  }
});

test.afterAll(async () => {
  if (server) {
    await server.close();
  }
});

test('menu interaction updates transcript and backend', async ({ page }) => {
  test.skip(skipSuite, skipReason);
  await page.goto(server.baseUrl + '/');
  await expect(page.locator('#status')).toContainText('Connected');
  await page.getByRole('button', { name: 'About' }).click();
  await expect(page.locator('#last-choice')).toHaveText('about');
  await expect(page.locator('#transcript li').nth(0)).toHaveText('User: Selected about');
  await expect(page.locator('#transcript li').nth(1)).toHaveText('Bot: Responding to about');
  expect(server.events).toHaveLength(1);
  expect(server.events[0].payload.choice).toBe('about');
});

test('sending chat messages echoes response', async ({ page }) => {
  test.skip(skipSuite, skipReason);
  await page.goto(server.baseUrl + '/');
  await expect(page.locator('#status')).toContainText('Connected');
  await page.fill('#chat-input', 'hello world');
  await page.click('#chat-send');
  const transcript = page.locator('#transcript li');
  await expect(transcript.nth(0)).toHaveText('User: hello world');
  await expect(transcript.nth(1)).toHaveText('Bot: echo: hello world');
});
