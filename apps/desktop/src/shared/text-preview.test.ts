import assert from 'node:assert/strict'
import { test } from 'node:test'
import { canPreview } from './text-preview.ts'

const file = (name: string, size = 5) => ({ id: 'id', name, size, sha256: 'a'.repeat(64) })
test('previews bounded text files, not images, extensionless names or oversized data', () => {
  assert.equal(canPreview(file('README.MD')), true)
  assert.equal(canPreview(file('app.ts')), true)
  assert.equal(canPreview(file('md')), false)
  assert.equal(canPreview(file('photo.png')), false)
  assert.equal(canPreview(file('big.txt', 1024 * 1024 + 1)), false)
  assert.equal(canPreview(file('bad.txt', -1)), false)
  assert.equal(canPreview({ id: 'upload', name: 'notes.md', size: 12 }), true)
})
