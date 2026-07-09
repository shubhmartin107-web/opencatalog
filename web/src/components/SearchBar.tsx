import { useState } from 'react'
import { useNavigate } from 'react-router-dom'

const API = '/api/v1'

export default function SearchBar() {
  const [query, setQuery] = useState('')
  const [results, setResults] = useState<any[]>([])
  const [open, setOpen] = useState(false)
  const navigate = useNavigate()

  async function handleInput(v: string) {
    setQuery(v)
    if (v.length < 2) { setOpen(false); return }
    const res = await fetch(`${API}/search?q=${encodeURIComponent(v)}`)
    const data = await res.json()
    setResults(data.results || [])
    setOpen(true)
  }

  return (
    <div style={{ position: 'relative' }}>
      <input value={query} onChange={e => handleInput(e.target.value)}
        placeholder="Search datasets, columns, terms..."
        style={{ width: '100%', padding: '8px 12px', borderRadius: 8, border: '1px solid #333',
          background: '#0f3460', color: '#eee', fontSize: 13 }}
      />
      {open && results.length > 0 && (
        <div style={{ position: 'absolute', top: 40, left: 0, right: 0, background: '#1a1a2e',
          border: '1px solid #333', borderRadius: 8, maxHeight: 300, overflow: 'auto', zIndex: 100 }}>
          {results.slice(0, 10).map((r: any, i: number) => (
            <div key={i} onClick={() => {
              if (r.dataset_id) navigate(`/datasets/${r.dataset_id}`)
              setOpen(false); setQuery(r.name)
            }} style={{ padding: '8px 12px', cursor: 'pointer', borderBottom: '1px solid #222',
              fontSize: 13 }}>
              <span style={{ color: '#e94560', fontSize: 11, marginRight: 8 }}>{r.kind}</span>
              {r.name}
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
