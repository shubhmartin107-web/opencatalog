import { useEffect, useState } from 'react'
import { useParams, Link } from 'react-router-dom'

const API = '/api/v1'

export default function DatasetDetail() {
  const { id } = useParams()
  const [ds, setDs] = useState<any>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    if (!id) return
    fetch(`${API}/datasets/${id}`).then(r => r.json()).then(data => {
      setDs(data)
      setLoading(false)
    })
  }, [id])

  if (loading) return <div style={{ color: '#888' }}>Loading...</div>
  if (!ds) return <div>Dataset not found</div>

  const cols = ds.schema || []

  return (
    <div>
      <Link to="/datasets" style={{ color: '#888', fontSize: 13, textDecoration: 'none' }}>← Back to datasets</Link>
      <h2 style={{ color: '#e94560', margin: '12px 0' }}>{ds.name}</h2>
      <div style={{ display: 'flex', gap: 16, marginBottom: 20, fontSize: 13, color: '#888' }}>
        <span>{ds.dataset_type}</span>
        {ds.classification && <span style={{ color: '#e94560' }}>{ds.classification}</span>}
        <Link to={`/lineage/${ds.id}`} style={{ color: '#0f3460', background: '#e94560', padding: '2px 12px', borderRadius: 4, textDecoration: 'none', fontSize: 12 }}>View Lineage</Link>
      </div>
      {ds.description && <p style={{ color: '#aaa', fontSize: 13, marginBottom: 16 }}>{ds.description}</p>}
      <h3 style={{ marginBottom: 12, fontSize: 15 }}>Schema ({cols.length} columns)</h3>
      <div style={{ overflowX: 'auto' }}>
        <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 13 }}>
          <thead>
            <tr style={{ borderBottom: '1px solid #333', color: '#888' }}>
              <th style={{ textAlign: 'left', padding: '8px 12px' }}>#</th>
              <th style={{ textAlign: 'left', padding: '8px 12px' }}>Name</th>
              <th style={{ textAlign: 'left', padding: '8px 12px' }}>Type</th>
              <th style={{ textAlign: 'left', padding: '8px 12px' }}>Nullable</th>
              <th style={{ textAlign: 'left', padding: '8px 12px' }}>Classification</th>
              <th style={{ textAlign: 'left', padding: '8px 12px' }}>Description</th>
            </tr>
          </thead>
          <tbody>
            {cols.map((col: any, i: number) => (
              <tr key={i} style={{ borderBottom: '1px solid #222' }}>
                <td style={{ padding: '8px 12px', color: '#555' }}>{col.ordinal_position}</td>
                <td style={{ padding: '8px 12px', fontWeight: 500 }}>{col.name}</td>
                <td style={{ padding: '8px 12px', color: '#888' }}>{col.column_type}</td>
                <td style={{ padding: '8px 12px', color: '#888' }}>{col.is_nullable ? 'YES' : 'NO'}</td>
                <td style={{ padding: '8px 12px' }}>
                  {col.classification && <span style={{ background: '#e94560', color: '#fff', padding: '2px 8px', borderRadius: 4, fontSize: 11 }}>{col.classification}</span>}
                </td>
                <td style={{ padding: '8px 12px', color: '#aaa' }}>{col.description}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  )
}
