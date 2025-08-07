import React from 'react'
import { Box } from '@mui/material'
import OrganizationDashboard from '../organization/OrganizationDashboard'

const OrganizationTab: React.FC = () => {
  return (
    <Box sx={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      <OrganizationDashboard />
    </Box>
  )
}

export default OrganizationTab
