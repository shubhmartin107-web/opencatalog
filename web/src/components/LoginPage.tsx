import { useState } from 'react'
import { useNavigate } from 'react-router-dom'

export default function LoginPage() {
  const [apiKey, setApiKey] = useState('')
  const [error, setError] = useState('')
  const navigate = useNavigate()

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    if (!apiKey.trim()) {
      setError('Please enter an API key')
      return
    }
    localStorage.setItem('catalog_api_key', apiKey.trim())
    navigate('/datasets')
  }

  return (
    <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', minHeight: '60vh' }}>
      <form onSubmit={handleSubmit} style={{ background: '#1a1a2e', padding: 40, borderRadius: 12, width: 360 }}>
        <h2 style={{ color: '#e94560', marginBottom: 24 }}>OpenCatalog Login</h2>
        <div style={{ marginBottom: 16 }}>
          <label style={{ display: 'block', color: '#aaa', marginBottom: 6, fontSize: 13 }}>API Key</label>
          <input
            type="password" value={apiKey}
            onChange={e => { setApiKey(e.target.value); setError('') }}
            placeholder="Enter your API key"
            style={{ width: '100%', padding: '10px 12px', background: '#16213e', border: '1px solid #333', borderRadius: 6, color: '#eee', fontSize: 14, boxSizing: 'border-box' }}
          />
        </div>
        {error && <p style={{ color: '#e94560', fontSize: 13, marginBottom: 12 }}>{error}</p>}
        <button type="submit" style={{ width: '100%', padding: '10px', background: '#e94560', color: '#fff', border: 'none', borderRadius: 6, fontSize: 14, cursor: 'pointer' }}>Sign In</button>
      </form>
    </div>
  )
}
