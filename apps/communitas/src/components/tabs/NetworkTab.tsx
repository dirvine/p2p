import React, { useState, useEffect } from 'react'
import {
  Grid,
  Card,
  CardContent,
  Typography,
  Box,
  LinearProgress,
  Chip,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Paper,
  Divider,
  Alert,
} from '@mui/material'
import {
  NetworkCheck,
  Router,
  Speed,
  SignalWifi4Bar,
  SignalWifiOff,
  Hub,
  Security,
} from '@mui/icons-material'

interface PeerConnection {
  id: string
  address: string
  latency: number
  status: 'Connected' | 'Connecting' | 'Disconnected' | 'Failed'
  natType: 'Direct' | 'STUN' | 'TURN' | 'Relay'
  connectionQuality: number // 0-100
  bandwidth: { up: number; down: number }
  lastSeen: Date
}

interface NetworkMetrics {
  bandwidth_up: number
  bandwidth_down: number
  packet_loss: number
  jitter: number
  nat_type: 'Open' | 'Moderate' | 'Strict' | 'Unknown'
  upnp_available: boolean
  ipv6_support: boolean
  total_connections: number
  active_connections: number
}

const NetworkTab: React.FC = () => {
  const [metrics, setMetrics] = useState<NetworkMetrics>({
    bandwidth_up: 850,
    bandwidth_down: 1200,
    packet_loss: 0.1,
    jitter: 5.2,
    nat_type: 'Moderate',
    upnp_available: true,
    ipv6_support: true,
    total_connections: 8,
    active_connections: 6,
  })

  const [peers] = useState<PeerConnection[]>([
    {
      id: '1',
      address: 'warm-ocean-breeze',
      latency: 23,
      status: 'Connected',
      natType: 'Direct',
      connectionQuality: 95,
      bandwidth: { up: 245, down: 780 },
      lastSeen: new Date(),
    },
    {
      id: '2',
      address: 'bright-mountain-peak',
      latency: 45,
      status: 'Connected',
      natType: 'STUN',
      connectionQuality: 87,
      bandwidth: { up: 180, down: 650 },
      lastSeen: new Date(),
    },
    {
      id: '3',
      address: 'gentle-river-flow',
      latency: 67,
      status: 'Connected',
      natType: 'Direct',
      connectionQuality: 78,
      bandwidth: { up: 320, down: 420 },
      lastSeen: new Date(),
    },
    {
      id: '4',
      address: 'dancing-forest-leaf',
      latency: 89,
      status: 'Connecting',
      natType: 'TURN',
      connectionQuality: 45,
      bandwidth: { up: 50, down: 120 },
      lastSeen: new Date(Date.now() - 30000),
    },
    {
      id: '5',
      address: 'silent-snow-crystal',
      latency: 156,
      status: 'Failed',
      natType: 'Relay',
      connectionQuality: 0,
      bandwidth: { up: 0, down: 0 },
      lastSeen: new Date(Date.now() - 300000),
    },
    {
      id: '6',
      address: 'endless-sky-horizon',
      latency: 234,
      status: 'Connected',
      natType: 'TURN',
      connectionQuality: 42,
      bandwidth: { up: 78, down: 145 },
      lastSeen: new Date(),
    },
  ])

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'Connected': return 'success'
      case 'Connecting': return 'warning'
      case 'Disconnected': return 'default'
      case 'Failed': return 'error'
      default: return 'default'
    }
  }

  const getNatTypeColor = (natType: string) => {
    switch (natType) {
      case 'Direct': return 'success'
      case 'STUN': return 'info'
      case 'TURN': return 'warning'
      case 'Relay': return 'error'
      default: return 'default'
    }
  }

  const getNatDifficulty = (natType: string): { text: string; severity: 'success' | 'info' | 'warning' | 'error' } => {
    switch (natType) {
      case 'Open': return { text: 'Excellent connectivity - Direct peer connections possible', severity: 'success' }
      case 'Moderate': return { text: 'Good connectivity - Some NAT traversal required', severity: 'info' }
      case 'Strict': return { text: 'Limited connectivity - Relay servers may be needed', severity: 'warning' }
      case 'Unknown': return { text: 'Connectivity unknown - Testing in progress', severity: 'error' }
      default: return { text: 'Unknown NAT configuration', severity: 'error' }
    }
  }

  const getQualityIcon = (quality: number) => {
    if (quality >= 80) return <SignalWifi4Bar color="success" />
    if (quality >= 60) return <SignalWifi4Bar color="warning" />
    if (quality >= 40) return <SignalWifi4Bar color="error" />
    return <SignalWifiOff color="error" />
  }

  // Simulate real-time updates
  useEffect(() => {
    const interval = setInterval(() => {
      setMetrics(prev => ({
        ...prev,
        bandwidth_up: prev.bandwidth_up + (Math.random() - 0.5) * 100,
        bandwidth_down: prev.bandwidth_down + (Math.random() - 0.5) * 150,
        active_connections: Math.max(4, Math.min(8, prev.active_connections + Math.floor((Math.random() - 0.5) * 2))),
      }))
    }, 5000)

    return () => clearInterval(interval)
  }, [])

  const natDiagnostic = getNatDifficulty(metrics.nat_type)

  return (
    <Box>
      <Typography variant="h4" gutterBottom sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
        <NetworkCheck />
        Network Diagnostics
      </Typography>
      
      {/* NAT & Connectivity Status */}
      <Card sx={{ mb: 3 }}>
        <CardContent>
          <Typography variant="h6" gutterBottom sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
            <Security />
            NAT & Connectivity Analysis
          </Typography>
          <Alert severity={natDiagnostic.severity} sx={{ mb: 2 }}>
            <strong>NAT Type: {metrics.nat_type}</strong> - {natDiagnostic.text}
          </Alert>
          <Grid container spacing={2}>
            <Grid item xs={12} md={4}>
              <Box sx={{ textAlign: 'center', p: 2 }}>
                <Typography variant="body2" color="textSecondary">UPnP Support</Typography>
                <Chip 
                  label={metrics.upnp_available ? 'Available' : 'Not Available'} 
                  color={metrics.upnp_available ? 'success' : 'error'}
                  sx={{ mt: 1 }}
                />
              </Box>
            </Grid>
            <Grid item xs={12} md={4}>
              <Box sx={{ textAlign: 'center', p: 2 }}>
                <Typography variant="body2" color="textSecondary">IPv6 Support</Typography>
                <Chip 
                  label={metrics.ipv6_support ? 'Enabled' : 'Disabled'} 
                  color={metrics.ipv6_support ? 'success' : 'warning'}
                  sx={{ mt: 1 }}
                />
              </Box>
            </Grid>
            <Grid item xs={12} md={4}>
              <Box sx={{ textAlign: 'center', p: 2 }}>
                <Typography variant="body2" color="textSecondary">Active Connections</Typography>
                <Typography variant="h6" sx={{ mt: 1 }}>
                  {metrics.active_connections}/{metrics.total_connections}
                </Typography>
              </Box>
            </Grid>
          </Grid>
        </CardContent>
      </Card>

      <Grid container spacing={3}>
        {/* Network Performance */}
        <Grid item xs={12} md={6}>
          <Card>
            <CardContent>
              <Typography variant="h6" gutterBottom sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                <Speed />
                Network Performance
              </Typography>
              <Box sx={{ mb: 2 }}>
                <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <Typography variant="body2">Upload</Typography>
                  <Typography variant="body2" fontWeight="bold">{Math.round(metrics.bandwidth_up)} kbps</Typography>
                </Box>
                <LinearProgress
                  variant="determinate"
                  value={Math.min(100, (metrics.bandwidth_up / 2000) * 100)}
                  sx={{ mt: 1, height: 8, borderRadius: 4 }}
                />
              </Box>
              <Box sx={{ mb: 2 }}>
                <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <Typography variant="body2">Download</Typography>
                  <Typography variant="body2" fontWeight="bold">{Math.round(metrics.bandwidth_down)} kbps</Typography>
                </Box>
                <LinearProgress
                  variant="determinate"
                  value={Math.min(100, (metrics.bandwidth_down / 2000) * 100)}
                  sx={{ mt: 1, height: 8, borderRadius: 4 }}
                />
              </Box>
              <Divider sx={{ my: 2 }} />
              <Grid container spacing={2}>
                <Grid item xs={6}>
                  <Typography variant="body2" color="textSecondary">Packet Loss</Typography>
                  <Typography variant="h6" color={metrics.packet_loss > 1 ? 'error' : 'success'}>
                    {metrics.packet_loss}%
                  </Typography>
                </Grid>
                <Grid item xs={6}>
                  <Typography variant="body2" color="textSecondary">Jitter</Typography>
                  <Typography variant="h6" color={metrics.jitter > 10 ? 'error' : 'success'}>
                    {metrics.jitter} ms
                  </Typography>
                </Grid>
              </Grid>
            </CardContent>
          </Card>
        </Grid>
        
        {/* Peer Connection Graph */}
        <Grid item xs={12} md={6}>
          <Card>
            <CardContent>
              <Typography variant="h6" gutterBottom sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                <Hub />
                Peer Connection Topology
              </Typography>
              <Box sx={{ textAlign: 'center', p: 2, position: 'relative', minHeight: 200 }}>
                {/* Simple visual representation of peer connections */}
                <Box sx={{ 
                  display: 'flex', 
                  flexDirection: 'column', 
                  alignItems: 'center',
                  gap: 2 
                }}>
                  {/* Central node (you) */}
                  <Box sx={{ 
                    width: 60, 
                    height: 60, 
                    borderRadius: '50%', 
                    bgcolor: 'primary.main', 
                    display: 'flex', 
                    alignItems: 'center', 
                    justifyContent: 'center',
                    color: 'white',
                    fontWeight: 'bold'
                  }}>
                    YOU
                  </Box>
                  
                  {/* Connection lines and peer nodes */}
                  <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 1, justifyContent: 'center' }}>
                    {peers.filter(p => p.status === 'Connected').map((peer) => (
                      <Box key={peer.id} sx={{ 
                        display: 'flex', 
                        flexDirection: 'column', 
                        alignItems: 'center',
                        minWidth: 80
                      }}>
                        <Box sx={{ 
                          width: 40, 
                          height: 40, 
                          borderRadius: '50%', 
                          bgcolor: peer.connectionQuality > 70 ? 'success.main' : peer.connectionQuality > 40 ? 'warning.main' : 'error.main',
                          display: 'flex', 
                          alignItems: 'center', 
                          justifyContent: 'center',
                          color: 'white',
                          fontSize: '0.75rem'
                        }}>
                          {peer.latency}ms
                        </Box>
                        <Typography variant="caption" sx={{ mt: 0.5, textAlign: 'center' }}>
                          {peer.address.split('-')[0]}
                        </Typography>
                      </Box>
                    ))}
                  </Box>
                </Box>
              </Box>
            </CardContent>
          </Card>
        </Grid>
        
        {/* Detailed Peer List */}
        <Grid item xs={12}>
          <Card>
            <CardContent>
              <Typography variant="h6" gutterBottom sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                <Router />
                Detailed Peer Connections
              </Typography>
              <TableContainer component={Paper} variant="outlined">
                <Table>
                  <TableHead>
                    <TableRow>
                      <TableCell>Quality</TableCell>
                      <TableCell>Address</TableCell>
                      <TableCell>Status</TableCell>
                      <TableCell>NAT Type</TableCell>
                      <TableCell>Latency</TableCell>
                      <TableCell>Bandwidth</TableCell>
                      <TableCell>Last Seen</TableCell>
                    </TableRow>
                  </TableHead>
                  <TableBody>
                    {peers.map((peer) => (
                      <TableRow key={peer.id} sx={{ 
                        backgroundColor: peer.status === 'Failed' ? 'error.light' : 'inherit',
                        opacity: peer.status === 'Failed' ? 0.6 : 1
                      }}>
                        <TableCell>
                          <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                            {getQualityIcon(peer.connectionQuality)}
                            <Typography variant="body2">{peer.connectionQuality}%</Typography>
                          </Box>
                        </TableCell>
                        <TableCell sx={{ fontFamily: 'monospace' }}>{peer.address}</TableCell>
                        <TableCell>
                          <Chip
                            label={peer.status}
                            color={getStatusColor(peer.status)}
                            size="small"
                          />
                        </TableCell>
                        <TableCell>
                          <Chip
                            label={peer.natType}
                            color={getNatTypeColor(peer.natType)}
                            size="small"
                            variant="outlined"
                          />
                        </TableCell>
                        <TableCell>{peer.latency}ms</TableCell>
                        <TableCell>
                          <Typography variant="body2">
                            ↑{peer.bandwidth.up} ↓{peer.bandwidth.down} kbps
                          </Typography>
                        </TableCell>
                        <TableCell>
                          <Typography variant="body2" color="textSecondary">
                            {peer.lastSeen.toLocaleTimeString()}
                          </Typography>
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </TableContainer>
            </CardContent>
          </Card>
        </Grid>
      </Grid>
    </Box>
  )
}

export default NetworkTab
