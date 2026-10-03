// End-to-end smoke test: drives the main flows over HTTP through the Vite dev server's
// proxy (so it exercises the same /api paths and proxy setup the browser uses).
//
//   API_PROXY_TARGET=http://127.0.0.1:3101 bun run dev --port 5181   # in one shell
//   SMOKE_BASE_URL=http://127.0.0.1:5181 bun run smoke              # in another
import type { Capsule, CreateProfileResult, Media, MeasurementSeries, Memory, Member, Session, TimelinePage } from '../src/api/types'

const BASE = process.env.SMOKE_BASE_URL ?? 'http://127.0.0.1:5173'
let passed = 0

function check(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(`FAILED: ${message}`)
  passed += 1
  console.log(`  ok  ${message}`)
}

async function call<T>(method: string, path: string, token: string | null, body?: unknown): Promise<{ status: number; data: T }> {
  const headers: Record<string, string> = {}
  if (token) headers.Authorization = `Bearer ${token}`
  if (body !== undefined) headers['Content-Type'] = 'application/json'
  const response = await fetch(BASE + path, { method, headers, body: body === undefined ? undefined : JSON.stringify(body) })
  const text = await response.text()
  return { status: response.status, data: (text ? JSON.parse(text) : undefined) as T }
}

async function uploadFiles(path: string, token: string, files: { name: string; type: string; bytes: Uint8Array }[]) {
  const form = new FormData()
  for (const file of files) form.append('file', new Blob([file.bytes], { type: file.type }), file.name)
  const response = await fetch(BASE + path, { method: 'POST', headers: { Authorization: `Bearer ${token}` }, body: form })
  return { status: response.status, data: (await response.json()) as Media[] }
}

const PNG = Uint8Array.from(
  atob('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII='),
  (c) => c.charCodeAt(0),
)
const PDF = new TextEncoder().encode('%PDF-1.4\n1 0 obj\n<< /Type /Catalog >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF\n')

async function main() {
  console.log(`Smoke test against ${BASE}`)
  const stamp = Date.now()

  const health = await fetch(`${BASE}/health`)
  check(health.ok, '/health is proxied and healthy')

  // --- Auth ---
  const password = 'correct horse battery'
  const reg = await call<Session>('POST', '/api/auth/register', null, {
    email: `ada+${stamp}@example.com`,
    password,
    first_name: 'Ada',
    last_name: 'Lovelace',
    date_of_birth: '1990-12-10',
  })
  check(reg.status === 201 && reg.data.token.length === 64, 'register returns a session')
  const token = reg.data.token
  const viewerReg = await call<Session>('POST', '/api/auth/register', null, {
    email: `grace+${stamp}@example.com`,
    password,
    first_name: 'Grace',
    last_name: 'Hopper',
    date_of_birth: null,
  })
  check(viewerReg.status === 201, 'second user registers')
  const viewerToken = viewerReg.data.token

  const login = await call<Session>('POST', '/api/auth/login', null, { email: `ADA+${stamp}@example.com`, password })
  check(login.status === 200, 'login works (email is case-insensitive)')
  const badLogin = await call<{ error: { code: string } }>('POST', '/api/auth/login', null, { email: `ada+${stamp}@example.com`, password: 'nope-nope' })
  check(badLogin.status === 401 && badLogin.data.error.code === 'invalid_credentials', 'wrong password → 401 invalid_credentials')
  check((await call('GET', '/api/users/me', null)).status === 401, 'no token → 401')

  // --- Profile + relationship ---
  const created = await call<CreateProfileResult>('POST', '/api/profiles', token, {
    profile_type: 'CHILD',
    name: 'Mira',
    date_of_birth: '2024-03-01',
    relationship_type: 'PARENT_CHILD',
    started_at: '2024-03-01',
  })
  check(created.status === 201 && created.data.relationship.role === 'OWNER', 'create profile + relationship (owner)')
  const profileId = created.data.profile.id
  const relId = created.data.relationship.id
  const rels = await call<{ id: number }[]>('GET', '/api/relationships', token)
  check(rels.data.some((r) => r.id === relId), 'relationship listed on dashboard')

  const added = await call<Member[]>('POST', `/api/relationships/${relId}/members`, token, { email: `grace+${stamp}@example.com`, role: 'VIEWER' })
  check(added.status === 201 && added.data.length === 2, 'add member by email as VIEWER')

  // --- Memory + media ---
  const memory = await call<Memory>('POST', `/api/relationships/${relId}/memories`, token, {
    category: 'MILESTONE',
    title: 'First steps',
    description: 'Across the living room',
    memory_date: '2025-03-14',
    tags: ['walking', 'Home'],
  })
  check(memory.status === 201 && memory.data.tags.length === 2, 'create memory with tags')
  const memoryId = memory.data.id

  const uploaded = await uploadFiles(`/api/memories/${memoryId}/media`, token, [
    { name: 'steps.png', type: 'image/png', bytes: PNG },
    { name: 'report.pdf', type: 'application/pdf', bytes: PDF },
  ])
  check(uploaded.status === 201 && uploaded.data.length === 2, 'multipart upload of image + PDF through the proxy')
  check(uploaded.data[0]?.media_type === 'IMAGE' && uploaded.data[1]?.media_type === 'DOCUMENT', 'media types detected from content')

  const withMedia = await call<Memory>('GET', `/api/memories/${memoryId}`, token)
  check(withMedia.data.media.length === 2, 'memory lists its media')
  const image = withMedia.data.media[0]!
  const content = await fetch(BASE + image.content_url, { headers: { Authorization: `Bearer ${token}` } })
  const bytes = new Uint8Array(await content.arrayBuffer())
  check(content.status === 200 && content.headers.get('content-type') === 'image/png', 'media content served via proxy with token')
  check(bytes.length === PNG.length && bytes.every((b, i) => b === PNG[i]), 'downloaded bytes match the upload')
  check((await fetch(BASE + image.content_url)).status === 401, 'media content without token → 401 (hence blob URLs in the UI)')
  const viewerContent = await fetch(BASE + image.content_url, { headers: { Authorization: `Bearer ${viewerToken}` } })
  check(viewerContent.status === 200, 'viewer member can read memory media')

  const badUpload = await uploadFiles(`/api/memories/${memoryId}/media`, token, [
    { name: 'evil.svg', type: 'image/svg+xml', bytes: new TextEncoder().encode('<svg xmlns="http://www.w3.org/2000/svg"/>') },
  ])
  check(badUpload.status === 400, 'SVG upload rejected with 400')

  // --- Timeline filters (the query the timeline screen builds) ---
  await call('POST', `/api/relationships/${relId}/memories`, token, { category: 'BIRTHDAY', title: 'First birthday', description: null, memory_date: '2025-03-01', tags: ['cake'] })
  const params = new URLSearchParams({ from: '2025-01-01', to: '2025-12-31', category: 'MILESTONE', tag: 'WALKING', q: 'living', order: 'desc', limit: '20', offset: '0' })
  const filtered = await call<TimelinePage>('GET', `/api/relationships/${relId}/memories?${params}`, token)
  check(filtered.status === 200 && filtered.data.total === 1 && filtered.data.memories[0]?.id === memoryId, 'timeline filters combine')
  const all = await call<TimelinePage>('GET', `/api/relationships/${relId}/memories?limit=1&offset=0`, token)
  check(all.data.total === 2 && all.data.memories.length === 1, 'timeline pagination reports total')
  const tags = await call<{ name: string }[]>('GET', `/api/relationships/${relId}/tags`, token)
  check(tags.data.map((t) => t.name).join(',') === 'cake,Home,walking', 'relationship tag suggestions')

  // --- Role enforcement ---
  const viewerWrite = await call('POST', `/api/relationships/${relId}/memories`, viewerToken, { category: 'GENERAL', title: 'x', memory_date: '2025-01-01' })
  check(viewerWrite.status === 403, 'viewer cannot create memories (403)')
  const viewerTimeline = await call<TimelinePage>('GET', `/api/relationships/${relId}/memories`, viewerToken)
  check(viewerTimeline.status === 200 && viewerTimeline.data.total === 2, 'viewer can read the timeline')

  // --- Time capsule ---
  const unlockAt = new Date(Date.now() + 3600_000).toISOString()
  const capsule = await call<Capsule>('POST', `/api/relationships/${relId}/capsules`, token, { title: 'For your 18th', message: 'Sealed words', unlock_at: unlockAt })
  check(capsule.status === 201 && capsule.data.status === 'LOCKED', 'create locked capsule')
  check(!('message' in capsule.data), 'create response does not echo the message')
  const capsuleMedia = await uploadFiles(`/api/capsules/${capsule.data.id}/media`, token, [{ name: 'sealed.png', type: 'image/png', bytes: PNG }])
  check(capsuleMedia.status === 201, 'upload media into the locked capsule')
  const fetched = await call<Capsule>('GET', `/api/capsules/${capsule.data.id}`, token)
  check(fetched.data.status === 'LOCKED' && !('message' in fetched.data) && !('media' in fetched.data) && fetched.data.media_count === 1, 'locked capsule returns metadata only')
  const sealed = await fetch(BASE + capsuleMedia.data[0]!.content_url, { headers: { Authorization: `Bearer ${token}` } })
  check(sealed.status === 404, 'sealed media content → 404 even for its uploader')
  check((await call('POST', `/api/capsules/${capsule.data.id}/open`, token)).status === 409, 'opening a locked capsule → 409')
  const edited = await call<Capsule>('PUT', `/api/capsules/${capsule.data.id}`, token, { title: 'For your 18th birthday' })
  check(edited.status === 200 && edited.data.title === 'For your 18th birthday', 'edit locked capsule (partial update)')
  const listed = await call<Capsule[]>('GET', `/api/relationships/${relId}/capsules`, viewerToken)
  check(listed.data.length === 1, 'viewer sees capsule in the list')

  // A capsule that unlocks in a few seconds: AVAILABLE → open → content visible.
  const soon = await call<Capsule>('POST', `/api/relationships/${relId}/capsules`, token, {
    title: 'Soon',
    message: 'Hello from the past',
    unlock_at: new Date(Date.now() + 3000).toISOString(),
  })
  await uploadFiles(`/api/capsules/${soon.data.id}/media`, token, [{ name: 'soon.png', type: 'image/png', bytes: PNG }])
  await new Promise((resolve) => setTimeout(resolve, 3500))
  const available = await call<Capsule>('GET', `/api/capsules/${soon.data.id}`, viewerToken)
  check(available.data.status === 'AVAILABLE' && !('message' in available.data), 'capsule becomes AVAILABLE, still without content')
  const opened = await call<Capsule>('POST', `/api/capsules/${soon.data.id}/open`, viewerToken)
  check(opened.status === 200 && opened.data.status === 'OPENED' && opened.data.message === 'Hello from the past', 'viewer opens it and sees the message')
  const openedMedia = await fetch(BASE + opened.data.media![0]!.content_url, { headers: { Authorization: `Bearer ${viewerToken}` } })
  check(openedMedia.status === 200, 'opened capsule media is readable')

  // --- Measurements + development ---
  for (const [date, value] of [['2024-06-01', 66.5], ['2024-12-01', 72], ['2025-03-01', 76.125]] as const) {
    const m = await call('POST', `/api/profiles/${profileId}/measurements`, token, { measurement_type: 'HEIGHT', value, measurement_date: date })
    check(m.status === 201, `add HEIGHT ${value} cm on ${date}`)
  }
  await call('POST', `/api/profiles/${profileId}/measurements`, token, { measurement_type: 'WEIGHT', value: 9.4, measurement_date: '2025-03-01' })
  const series = await call<MeasurementSeries[]>('GET', `/api/profiles/${profileId}/measurements/series`, token)
  check(series.data.length === 2 && series.data[0]?.measurement_type === 'HEIGHT' && series.data[0].points.length === 3, 'chart series per measurement type')
  const tooEarly = await call('POST', `/api/profiles/${profileId}/measurements`, token, { measurement_type: 'HEIGHT', value: 50, measurement_date: '2020-01-01' })
  check(tooEarly.status === 400, 'measurement before birth date → 400')
  const record = await call('POST', `/api/profiles/${profileId}/development-records`, token, {
    record_date: '2025-03-01',
    notes: 'Birthday week',
    observations: [{ domain: 'MOTOR', observation: 'Took three steps' }, { domain: 'LANGUAGE', observation: 'Says "mama"' }],
  })
  check(record.status === 201, 'create development record with observations')
  const motor = await call<unknown[]>('GET', `/api/profiles/${profileId}/development-records?domain=MOTOR`, viewerToken)
  check(motor.status === 200 && motor.data.length === 1, 'viewer reads development records filtered by domain')

  // --- Logout ---
  check((await call('POST', '/api/auth/logout', token)).status === 204, 'logout')
  check((await call('GET', '/api/users/me', token)).status === 401, 'token is invalid after logout')

  console.log(`\nAll ${passed} checks passed.`)
}

main().catch((error: unknown) => {
  console.error(error instanceof Error ? error.message : error)
  process.exit(1)
})
