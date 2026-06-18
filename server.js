const express = require('express');
const crypto = require('crypto');
const path = require('path');

const app = express();
app.use(express.json());
const PORT = process.env.PORT || 3000;

// ── 内存状态 ────────────────────────────────

const agents = new Map();       // client_id → { password, online, last_seen }
const commands = new Map();     // client_id → [{ command_id, command }]
const waiters = new Map();      // command_id → resolve

function now() { return Math.floor(Date.now() / 1000); }

function genId() { return crypto.randomBytes(4).toString('hex'); }
function genPw() { return crypto.randomBytes(12).toString('base64url'); }

function verify(a, b) {
  if (a.length !== b.length) return false;
  return crypto.timingSafeEqual(Buffer.from(a), Buffer.from(b));
}

// ── API ──────────────────────────────────────

app.post('/register', (_req, res) => {
  const cid = genId();
  const pw = genPw();
  agents.set(cid, { password: pw, online: false, last_seen: now() });
  commands.set(cid, []);
  res.json({ client_id: cid, password: pw });
});

app.post('/agent/poll', (req, res) => {
  const { client_id, password } = req.body;
  const a = agents.get(client_id);
  if (!a || !verify(password, a.password)) return res.status(401).json({ error: 'auth' });

  a.online = true;
  a.last_seen = now();

  const q = commands.get(client_id) || [];
  const cmd = q.shift();
  if (cmd) return res.json({ command: cmd });
  res.json({ command: null });
});

app.post('/agent/result', (req, res) => {
  const { client_id, password, command_id, stdout, stderr, exit_code } = req.body;
  const a = agents.get(client_id);
  if (!a || !verify(password, a.password)) return res.status(401).json({ error: 'auth' });

  const resolve = waiters.get(command_id);
  if (resolve) {
    waiters.delete(command_id);
    resolve({ stdout, stderr, exit_code });
  }
  res.json({ status: 'ok' });
});

app.post('/exec', async (req, res) => {
  const { client_id, password, command } = req.body;
  const a = agents.get(client_id);
  if (!a || !verify(password, a.password)) return res.status(401).json({ error: 'auth' });
  if (!a.online) return res.status(503).json({ error: 'agent offline' });
  if (/[\x00\x1b]/.test(command)) return res.status(400).json({ error: 'binary rejected' });

  const command_id = crypto.randomUUID();
  const q = commands.get(client_id) || [];
  q.push({ command_id, command });
  commands.set(client_id, q);

  const result = await new Promise((resolve, reject) => {
    waiters.set(command_id, resolve);
    setTimeout(() => { waiters.delete(command_id); reject(new Error('timeout')); }, 30000);
  });

  res.json(result);
});

// ── 静态文件 ─────────────────────────────────

app.use(express.static(path.join(__dirname, 'public'), {
  setHeaders(res, fp) {
    if (fp.endsWith('.sh')) res.set('Content-Type', 'text/plain; charset=utf-8');
  }
}));

app.listen(PORT, () => console.log(`CloudSH → http://localhost:${PORT}`));
