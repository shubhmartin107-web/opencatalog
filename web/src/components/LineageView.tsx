import { useEffect, useState, useCallback } from 'react'
import { useParams, Link } from 'react-router-dom'
import {
  ReactFlow, Background, Controls, MiniMap, useNodesState, useEdgesState,
  MarkerType,
} from '@xyflow/react'
import '@xyflow/react/dist/style.css'

const API = '/api/v1'

export default function LineageView() {
  const { id } = useParams()
  const [graph, setGraph] = useState<any>(null)
  const [loading, setLoading] = useState(true)
  const [nodes, setNodes, onNodesChange] = useNodesState([])
  const [edges, setEdges, onEdgesChange] = useEdgesState([])

  useEffect(() => {
    if (!id) return
    fetch(`${API}/datasets/${id}/lineage`).then(r => r.json()).then(data => {
      setGraph(data)
      setLoading(false)
    })
  }, [id])

  useEffect(() => {
    if (!graph) return
    const flowNodes = (graph.nodes || []).map((n: any, i: number) => ({
      id: n.id,
      position: { x: 200 * i, y: 100 },
      data: { label: n.label },
      style: {
        background: '#1a1a2e', color: '#eee', border: '1px solid #e94560',
        borderRadius: 8, padding: '8px 16px', fontSize: 12,
      },
    }))
    const flowEdges = (graph.edges || []).map((e: any) => ({
      id: e.id,
      source: e.source_node_id,
      target: e.target_node_id,
      label: e.transformation_subtype,
      markerEnd: { type: MarkerType.ArrowClosed, color: '#e94560' },
      style: { stroke: '#555' },
      labelStyle: { fill: '#888', fontSize: 10 },
    }))
    setNodes(flowNodes)
    setEdges(flowEdges)
  }, [graph])

  if (loading) return <div style={{ color: '#888' }}>Loading lineage...</div>

  return (
    <div>
      <Link to={`/datasets/${id}`} style={{ color: '#888', fontSize: 13, textDecoration: 'none' }}>← Back to dataset</Link>
      <h2 style={{ color: '#e94560', margin: '12px 0' }}>Lineage Graph</h2>
      <div style={{ height: 500, border: '1px solid #333', borderRadius: 8 }}>
        <ReactFlow
          nodes={nodes} edges={edges}
          onNodesChange={onNodesChange} onEdgesChange={onEdgesChange}
          fitView
        >
          <Background color="#333" gap={20} />
          <Controls />
          <MiniMap style={{ background: '#1a1a2e' }} />
        </ReactFlow>
      </div>
    </div>
  )
}
