import { Routes, Route, Link, useLocation } from 'react-router-dom'
import DatasetsPage from './components/DatasetsPage'
import DatasetDetail from './components/DatasetDetail'
import LineageView from './components/LineageView'
import GlossaryPage from './components/GlossaryPage'
import PoliciesPage from './components/PoliciesPage'
import SearchBar from './components/SearchBar'
import LoginPage from './components/LoginPage'
import ApiKeyManager from './components/ApiKeyManager'

export async function apiFetch(path: string, options?: RequestInit) {
  const apiKey = localStorage.getItem('catalog_api_key')
  const headers: Record<string, string> = { 'Content-Type': 'application/json' }
  if (apiKey) headers['X-API-Key'] = apiKey
  const res = await fetch(`/api/v1${path}`, { ...options, headers: { ...headers, ...(options?.headers as Record<string, string> || {}) } })
  if (res.status === 401 || res.status === 403) {
    localStorage.removeItem('catalog_api_key')
    window.location.href = '/login'
    throw new Error('Unauthorized')
  }
  return res.json()
}

function Nav() {
  const location = useLocation()
  const apiKey = localStorage.getItem('catalog_api_key')
  const links = [
    { to: '/datasets', label: 'Datasets' },
    { to: '/glossary', label: 'Glossary' },
    { to: '/policies', label: 'Policies' },
  ]
  return (
    <nav style={{ background: '#1a1a2e', padding: '12px 24px', display: 'flex', alignItems: 'center', gap: 24 }}>
      <Link to="/" style={{ color: '#e94560', fontWeight: 700, fontSize: 20, textDecoration: 'none' }}>OpenCatalog</Link>
      <div style={{ flex: 1, maxWidth: 400 }}><SearchBar /></div>
      <div style={{ display: 'flex', gap: 16, alignItems: 'center' }}>
        {links.map(l => (
          <Link key={l.to} to={l.to} style={{
            color: location.pathname.startsWith(l.to) ? '#e94560' : '#ccc',
            textDecoration: 'none', fontWeight: 500, fontSize: 14
          }}>{l.label}</Link>
        ))}
        {apiKey ? (
          <>
            <Link to="/apikeys" style={{ color: '#ccc', textDecoration: 'none', fontSize: 13, fontWeight: 500 }}>Keys</Link>
            <span style={{ color: '#888', fontSize: 12, fontFamily: 'monospace' }}>{apiKey.slice(0, 8)}...</span>
          </>
        ) : (
          <Link to="/login" style={{ color: '#e94560', textDecoration: 'none', fontSize: 14, fontWeight: 600 }}>Login</Link>
        )}
      </div>
    </nav>
  )
}

export default function App() {
  return (
    <div style={{ minHeight: '100vh', background: '#16213e', color: '#eee', fontFamily: 'system-ui, sans-serif' }}>
      <Nav />
      <div style={{ padding: 24 }}>
        <Routes>
          <Route path="/" element={<HomePage />} />
          <Route path="/datasets" element={<DatasetsPage />} />
          <Route path="/datasets/:id" element={<DatasetDetail />} />
          <Route path="/lineage/:id" element={<LineageView />} />
          <Route path="/glossary" element={<GlossaryPage />} />
          <Route path="/policies" element={<PoliciesPage />} />
          <Route path="/login" element={<LoginPage />} />
          <Route path="/apikeys" element={<ApiKeyManager />} />
        </Routes>
      </div>
    </div>
  )
}

function HomePage() {
  return (
    <div style={{ textAlign: 'center', paddingTop: 80 }}>
      <h1 style={{ fontSize: 36, color: '#e94560' }}>OpenCatalog</h1>
      <p style={{ color: '#aaa', fontSize: 18, marginBottom: 40 }}>Automated Metadata Catalog with Column-Level Lineage & Policy Governance</p>
      <div style={{ display: 'flex', gap: 24, justifyContent: 'center', flexWrap: 'wrap' }}>
        {[
          { to: '/datasets', label: 'Browse Datasets', desc: 'Explore schemas, columns, and metadata' },
          { to: '/glossary', label: 'Business Glossary', desc: 'Manage terms, domains, and mappings' },
          { to: '/policies', label: 'Governance Policies', desc: 'Masking, access control, and auditing' },
        ].map(card => (
          <Link key={card.to} to={card.to} style={{
            background: '#1a1a2e', border: '1px solid #333', borderRadius: 12, padding: '32px 24px',
            width: 260, textDecoration: 'none', color: '#eee', transition: 'border-color 0.2s'
          }}>
            <h3 style={{ color: '#e94560', marginBottom: 8 }}>{card.label}</h3>
            <p style={{ color: '#888', fontSize: 13 }}>{card.desc}</p>
          </Link>
        ))}
      </div>
    </div>
  )
}
