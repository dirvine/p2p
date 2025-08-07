import React from 'react'
import { 
  Box,
  Typography,
  Card,
  CardContent,
  Alert,
} from '@mui/material'
import IdentityManager from '../identity/IdentityManager'

const IdentityTab: React.FC = () => {
  return (
    <Box>
      <Typography variant="h5" gutterBottom>
        Identity Management
      </Typography>
      
      <Alert severity="info" sx={{ mb: 3 }}>
        Manage your P2P identities, 4-word addresses, and secure key storage.
      </Alert>

      <Card>
        <CardContent>
          <IdentityManager />
        </CardContent>
      </Card>
    </Box>
  )
}

export default IdentityTab
