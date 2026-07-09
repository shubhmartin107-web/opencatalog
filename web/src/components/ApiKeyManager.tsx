import { useState } from 'react'

interface ApiKeyEntry {
  id: string
  name: string
  key: string
  created: string
}

export default function ApiKeyManager() {
  const [keys, setKeys] = useState<ApiKeyEntry[]>([
    { id: '1', name: 'Default Key', key: localStorage.getItem('catalog_api_key') || '', created: new Date().toISOString().slice(0, 10) },
  ])
  const [name, setName] = useState('')

  function generateKey() {
    if (!name.trim()) return
    const newKey: ApiKeyEntry = {
      id: crypto.randomUUID(),
      name: name.trim(),
      key: crypto.randomUUID(),
      created: new Date().toISOString().slice(0, 10),
    }
    setKeys(prev => [...prev, newKey])
    setName('')
  }

  function copyToClipboard(key: string) {
    navigator.clipboard.writeText(key)
  }

  function useAsCurrent(key: string) {
    localStorage.setItem('catalog_api_key', key)
    window.location.reload()
  }

  return (
    <div>
      <h2 style={{ color: '#e94560', marginBottom: 20 }}>API Key Manager</h2>

      <div style={{ background: '#1a1a2e', borderRadius: 12, padding: 24, marginBottom: 24 }}>
        <h3 style={{ color: '#eee', marginBottom: 16, fontSize: 16 }}>Generate New Key</h3>
        <div style={{ display: 'flex', gap: 12 }}>
          <input
            value={name} onChange={e => setName(e.target.value)}
            placeholder="Key name (e.g. CI/CD, local dev)"
            style={{ flex: 1, padding: '10px 12px', background: '#16213e', border: '1px solid #333', borderRadius: 6, color: '#eee', fontSize: 14 }}
            onKeyDown={e => e.key === 'Enter' && generateKey()}
          />
          <button onClick={generateKey} style={{ padding: '10px 20px', background: '#e94560', color: '#fff', border: 'none', borderRadius: 6, fontSize: 14, cursor: 'pointer' }}>Generate</button>
        </div>
      </div>

      <div style={{ background: '#1a1a2e', borderRadius: 12, padding: 24, marginBottom: 24 }}>
        <h3 style={{ color: '#eee', marginBottom: 16, fontSize: 16 }}>Your API Keys</h3>
        {keys.length === 0 ? (
          <p style={{ color: '#888', fontSize: 13 }}>No keys yet.</p>
        ) : (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
            {keys.map(entry => (
              <div key={entry.id} style={{ background: '#16213e', borderRadius: 8, padding: '12px 16px', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <div>
                  <div style={{ fontWeight: 600, fontSize: 14, marginBottom: 4 }}>{entry.name}</div>
                  <div style={{ fontSize: 12, color: '#888', fontFamily: 'monospace' }}>{entry.key.slice(0, 8)}...{entry.key.slice(-4)}</div>
                  <div style={{ fontSize: 11, color: '#555' }}>Created {entry.created}</div>
                </div>
                <div style={{ display: 'flex', gap: 8 }}>
                  <button onClick={() => copyToClipboard(entry.key)} style={{ padding: '6px 12px', background: 'transparent', border: '1px solid #555', borderRadius: 6, color: '#ccc', fontSize: 12, cursor: 'pointer' }}>Copy</button>
                  <button onClick={() => useAsCurrent(entry.key)} style={{ padding: '6px 12px', background: 'transparent', border: '1px solid #e94560', borderRadius: 6, color: '#e94560', fontSize: 12, cursor: 'pointer' }}>Use</button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      <div style={{ background: '#1a1a2e', borderRadius: 12, padding: 24 }}>
        <h3 style={{ color: '#eee', marginBottom: 16, fontSize: 16 }}>How to Use</h3>
        <p style={{ color: '#aaa', fontSize: 13, marginBottom: 12 }}>Include the API key in all requests via the <code style={{ background: '#16213e', padding: '2px 6px', borderRadius: 4 }}>X-API-Key</code> header:</p>
        <pre style={{ background: '#0f3460', padding: 16, borderRadius: 8, fontSize: 12, color: '#ccc', overflow: 'auto' }}>
{`curl -H "X-API-Key: YOUR_API_KEY" \\
  http://localhost:3000/api/v1/datasets`}
        </pre>
        <p style={{ color: '#aaa', fontSize: 13, marginTop: 12 }}>The Web UI stores your key in <code style={{ background: '#16213e', padding: '2px 6px', borderRadius: 4 }}>localStorage</code> under <code style={{ background: '#16213e', padding: '2px 6px', borderRadius: 4 }}>catalog_api_key</code>.</p>
      </div>
    </div>
  )
}
