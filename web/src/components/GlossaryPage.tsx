import { useEffect, useState } from 'react'

const API = '/api/v1'

export default function GlossaryPage() {
  const [terms, setTerms] = useState<any[]>([])
  const [loading, setLoading] = useState(true)
  const [name, setName] = useState('')
  const [desc, setDesc] = useState('')
  const [domain, setDomain] = useState('')

  useEffect(() => {
    loadTerms()
  }, [])

  async function loadTerms() {
    const res = await fetch(`${API}/glossary`)
    const data = await res.json()
    setTerms(data)
    setLoading(false)
  }

  async function createTerm() {
    const res = await fetch(`${API}/glossary`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name, description: desc, domain }),
    })
    if (res.ok) {
      setName(''); setDesc(''); setDomain('')
      loadTerms()
    }
  }

  if (loading) return <div style={{ color: '#888' }}>Loading glossary...</div>

  return (
    <div>
      <h2 style={{ color: '#e94560', marginBottom: 20 }}>Business Glossary</h2>

      <div style={{ display: 'flex', gap: 12, marginBottom: 24, flexWrap: 'wrap', alignItems: 'flex-end' }}>
        <div>
          <div style={{ fontSize: 12, color: '#888', marginBottom: 4 }}>Name</div>
          <input value={name} onChange={e => setName(e.target.value)}
            style={{ padding: '8px 12px', borderRadius: 6, border: '1px solid #333', background: '#0f3460', color: '#eee' }} />
        </div>
        <div>
          <div style={{ fontSize: 12, color: '#888', marginBottom: 4 }}>Description</div>
          <input value={desc} onChange={e => setDesc(e.target.value)}
            style={{ padding: '8px 12px', borderRadius: 6, border: '1px solid #333', background: '#0f3460', color: '#eee', width: 300 }} />
        </div>
        <div>
          <div style={{ fontSize: 12, color: '#888', marginBottom: 4 }}>Domain</div>
          <input value={domain} onChange={e => setDomain(e.target.value)}
            style={{ padding: '8px 12px', borderRadius: 6, border: '1px solid #333', background: '#0f3460', color: '#eee' }} />
        </div>
        <button onClick={createTerm} style={{ padding: '8px 20px', borderRadius: 6, border: 'none', background: '#e94560', color: '#fff', cursor: 'pointer' }}>
          Create Term
        </button>
      </div>

      <div style={{ display: 'grid', gap: 12 }}>
        {terms.map((t: any) => (
          <div key={t.id} style={{ background: '#1a1a2e', border: '1px solid #333', borderRadius: 8, padding: '16px 20px' }}>
            <div style={{ fontWeight: 600, marginBottom: 4 }}>{t.name}</div>
            <div style={{ fontSize: 13, color: '#aaa', marginBottom: 4 }}>{t.description}</div>
            <div style={{ display: 'flex', gap: 8, fontSize: 11, color: '#888' }}>
              {t.domain && <span style={{ background: '#0f3460', padding: '2px 8px', borderRadius: 4 }}>{t.domain}</span>}
              <span style={{ color: '#555' }}>{t.status}</span>
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}
