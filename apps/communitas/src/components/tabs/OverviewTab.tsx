import React from 'react'
import {
  Grid,
  Card,
  CardContent,
  Typography,
  Box,
  Chip,
} from '@mui/material'
import { NetworkHealth } from '../../types'

interface OverviewTabProps {
  networkHealth: NetworkHealth
}

const OverviewTab: React.FC<OverviewTabProps> = ({ networkHealth }) => {
  const getStatusColor = (status: string) => {
    switch (status) {
      case 'Connected': return 'success'
      case 'Connecting': return 'warning'
      case 'Disconnected': return 'error'
      default: return 'default'
    }
  }

  return (
    <Box>
      <Typography variant="h4" gutterBottom>
        Network Overview
      </Typography>
      
      <Grid container spacing={3}>
        <Grid item xs={12} md={6}>
          <Card>
            <CardContent>
              <Typography variant="h6" gutterBottom>
                Connection Status
              </Typography>
              <Chip 
                label={networkHealth.status}
                color={getStatusColor(networkHealth.status)}
                size="medium"
              />
            </CardContent>
          </Card>
        </Grid>
        
        <Grid item xs={12} md={6}>
          <Card>
            <CardContent>
              <Typography variant="h6" gutterBottom>
                Connected Peers
              </Typography>
              <Typography variant="h3">
                {networkHealth.peer_count}
              </Typography>
            </CardContent>
          </Card>
        </Grid>
        
        <Grid item xs={12} md={6}>
          <Card>
            <CardContent>
              <Typography variant="h6" gutterBottom>
                NAT Type
              </Typography>
              <Typography variant="h5">
                {networkHealth.nat_type}
              </Typography>
            </CardContent>
          </Card>
        </Grid>
        
        <Grid item xs={12} md={6}>
          <Card>
            <CardContent>
              <Typography variant="h6" gutterBottom>
                Network Performance
              </Typography>
              <Typography variant="body1">
                Bandwidth: {networkHealth.bandwidth_kbps} kbps
              </Typography>
              <Typography variant="body1">
                Latency: {networkHealth.avg_latency_ms} ms
              </Typography>
            </CardContent>
          </Card>
        </Grid>
      </Grid>
    </Box>
  )
}

export default OverviewTab
