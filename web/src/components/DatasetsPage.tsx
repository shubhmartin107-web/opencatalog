import { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'

const API = '/api/v1'

export default function DatasetsPage() {
  const [datasets, setDatasets] = useState<any[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    fetch(`${API}/datasets`).then(r => r.json()).then(data => {
      setDatasets(data)
      setLoading(false)
    })
  }, [])

  if (loading) return <div style={{ color: '#888' }}>Loading datasets...</div>

  return (
    <div>
      <h2 style={{ color: '#e94560', marginBottom: 20 }}>Datasets</h2>
      <div style={{ display: 'grid', gap: 12 }}>
        {datasets.map((ds: any) => (
          <Link key={ds.id} to={`/datasets/${ds.id}`} style={{
            background: '#1a1a2e', border: '1px solid #333', borderRadius: 8, padding: '16px 20px',
            textDecoration: 'none', color: '#eee', display: 'block'
          }}>
            <div style={{ fontWeight: 600, marginBottom: 4 }}>{ds.name}</div>
            <div style={{ display: 'flex', gap: 16, fontSize: 12, color: '#888' }}>
              <span>{(ds.schema || []).length} columns</span>
              <span>{ds.dataset_type}</span>
              {ds.classification && <span style={{ color: '#e94560' }}>{ds.classification}</span>}
              <span style={{ color: '#555' }}>{ds.tags?.join(', ')}</span>
            </div>
          </Link>
        ))}
      </div>
    </div>
  )
}
