// Development-only Worker launcher. It creates no hosted computers and accepts no provider keys.
import { spawn, spawnSync } from 'node:child_process'
import { createPrivateKey, createPublicKey, generateKeyPairSync } from 'node:crypto'
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { setTimeout as delay } from 'node:timers/promises'

const dir = process.env.CODYNC_STACK_DIR
const info = process.env.CODYNC_STACK_INFO
const port = Number(process.env.CODYNC_STACK_PORT)
if (!dir || !info || !Number.isInteger(port) || port < 1 || port > 65535) throw new Error('Missing local Worker settings')
mkdirSync(dir, { recursive: true })
const keyFile = join(dir, 'device-certificate.json')
const privateKey = existsSync(keyFile)
  ? createPrivateKey({ key: JSON.parse(readFileSync(keyFile, 'utf8')), format: 'jwk' })
  : generateKeyPairSync('ed25519').privateKey
if (!existsSync(keyFile)) writeFileSync(keyFile, JSON.stringify(privateKey.export({ format: 'jwk' })), { mode: 0o600 })
const publicKey = createPublicKey(privateKey).export({ format: 'jwk' }).x
const issuer = /\[env\.dev\][\s\S]*?CLERK_ISSUER = "([^"]+)"/.exec(readFileSync('wrangler.toml', 'utf8'))?.[1]
if (!issuer) throw new Error('Missing development issuer')
const response = await fetch(`${issuer}/.well-known/jwks.json`, { signal: AbortSignal.timeout(15000) })
if (!response.ok) throw new Error(`Development public keys: HTTP ${response.status}`)
const { keys } = await response.json()
const jwtKey = createPublicKey({ key: keys[0], format: 'jwk' }).export({ type: 'spki', format: 'pem' }).toString().replace(/-----[A-Z ]+-----|\s/g, '')
const wrangler = join(process.cwd(), 'node_modules/.bin/wrangler')
const migration = spawnSync(wrangler, ['d1', 'migrations', 'apply', 'codync-local', '--local', '--env', 'local', '--persist-to', dir], { stdio: 'inherit', env: { ...process.env, CI: '1' } })
if (migration.status !== 0) throw new Error('Local Worker migrations failed')
const child = spawn(wrangler, ['dev', '--env', 'local', '--ip', '127.0.0.1', '--port', String(port), '--persist-to', dir,
  '--var', `CLERK_ISSUER:${issuer}`, '--var', `CLERK_JWT_KEY:${jwtKey}`, '--var', `DEVICE_CERT_KEY:${JSON.stringify(privateKey.export({ format: 'jwk' }))}`], { stdio: 'inherit' })
let stopping = false
const stop = () => { stopping = true; child.kill('SIGTERM') }
process.on('SIGINT', stop)
process.on('SIGTERM', stop)
process.on('exit', () => child.kill('SIGTERM'))
child.on('error', error => { console.error(error.message); process.exit(1) })
child.on('exit', code => process.exit(stopping ? 0 : code || 1))
const url = `http://127.0.0.1:${port}`
let ready = false
for (let attempt = 0; attempt < 120; attempt++) {
  try { ready = (await fetch(`${url}/v1/health`, { signal: AbortSignal.timeout(1000) })).ok } catch { /* Readiness retries while Wrangler starts. */ }
  if (ready) break
  await delay(500)
}
if (!ready) { stop(); throw new Error('Local Worker did not become ready') }
writeFileSync(info, JSON.stringify({ url, certPub: publicKey }))
