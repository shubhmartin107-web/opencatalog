import { useEffect, useState } from 'react'

const API = '/api/v1'

export default function PoliciesPage() {
  const [policies, setPolicies] = useState<any[]>([])
  const [loading, setLoading] = useState(true)
  const [name, setName] = useState('')
  const [ptype, setPtype] = useState('masking')
  const [pattern, setPattern] = useState('*')
  const [action, setAction] = useState('hash')
  const [roles, setRoles] = useState('')

  useEffect(() => { loadPolicies() }, [])

  async function loadPolicies() {
    const res = await fetch(`${API}/policies`)
    const data = await res.json()
    setPolicies(data)
    setLoading(false)
  }

  async function createPolicy() {
    const res = await fetch(`${API}/policies`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        name, policy_type: ptype, dataset_pattern: pattern,
        action, roles: roles.split(',').map(r => r.trim()),
      }),
    })
    if (res.ok) { setName(''); setPattern('*'); loadPolicies() }
  }

  if (loading) return <div style={{ color: '#888' }}>Loading policies...</div>

  return (
    <div>
      <h2 style={{ color: '#e94560', marginBottom: 20 }}>Governance Policies</h2>

      <div style={{ display: 'flex', gap: 12, marginBottom: 24, flexWrap: 'wrap', alignItems: 'flex-end' }}>
        <div>
          <div style={{ fontSize: 12, color: '#888', marginBottom: 4 }}>Name</div>
          <input value={name} onChange={e => setName(e.target.value)} style={{ padding: '8px 12px', borderRadius: 6, border: '1px solid #333', background: '#0f3460', color: '#eee' }} />
        </div>
        <div>
          <div style={{ fontSize: 12, color: '#888', marginBottom: 4 }}>Type</div>
          <select value={ptype} onChange={e => setPtype(e.target.value)} style={{ padding: '8px 12px', borderRadius: 6, border: '1px solid #333', background: '#0f3460', color: '#eee' }}>
            <option value="masking">Masking</option>
            <option value="row_filter">Row Filter</option>
            <option value="access">Access</option>
          </select>
        </div>
        <div>
          <div style={{ fontSize: 12, color: '#888', marginBottom: 4 }}>Dataset Pattern</div>
          <input value={pattern} onChange={e => setPattern(e.target.value)} style={{ padding: '8px 12px', borderRadius: 6, border: '1px solid #333', background: '#0f3460', color: '#eee' }} />
        </div>
        <div>
          <div style={{ fontSize: 12, color: '#888', marginBottom: 4 }}>Action</div>
          <select value={action} onChange={e => setAction(e.target.value)} style={{ padding: '8px 12px', borderRadius: 6, border: '1px solid #333', background: '#0f3460', color: '#eee' }}>
            <option value="redact">Redact</option>
            <option value="hash">Hash</option>
            <option value="nullify">Nullify</option>
            <option value="deny">Deny</option>
          </select>
        </div>
        <div>
          <div style={{ fontSize: 12, color: '#888', marginBottom: 4 }}>Roles (comma-separated)</div>
          <input value={roles} onChange={e => setRoles(e.target.value)} placeholder="admin,analyst" style={{ padding: '8px 12px', borderRadius: 6, border: '1px solid #333', background: '#0f3460', color: '#eee' }} />
        </div>
        <button onClick={createPolicy} style={{ padding: '8px 20px', borderRadius: 6, border: 'none', background: '#e94560', color: '#fff', cursor: 'pointer' }}>Create</button>
      </div>

      <div style={{ display: 'grid', gap: 12 }}>
        {policies.map((p: any) => (
          <div key={p.id} style={{ background: '#1a1a2e', border: '1px solid #333', borderRadius: 8, padding: '16px 20px' }}>
            <div style={{ fontWeight: 600, marginBottom: 4 }}>{p.name}</div>
            <div style={{ fontSize: 13, color: '#aaa' }}>
              {p.policy_type} · {p.rules?.length} rule(s) · {p.enabled ? '✅ Enabled' : '⛔ Disabled'}
            </div>
            {p.rules?.map((r: any, i: number) => (
              <div key={i} style={{ fontSize: 12, color: '#888', marginTop: 4 }}>
                {r.dataset_pattern} → {Object.keys(r.action || {})[0]} (roles: {r.roles?.join(', ')})
              </div>
            ))}
          </div>
        ))}
      </div>
    </div>
  )
}
