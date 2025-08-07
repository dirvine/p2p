# Task 3: Organizational Hierarchy

## Overview
Implement a three-tier organizational structure: Organisations → Groups → Projects with proper permissions and file systems.

## Duration
18 hours

## Requirements

### Hierarchy Structure
- **Organisations**: Top-level entities with file systems and member management
- **Groups**: Chat-only entities within organizations (no file systems)
- **Projects**: Task-oriented entities with dedicated file systems
- Nested permissions and inheritance model

### Permission System
- Role-based access control (Owner, Admin, Member, Viewer)
- Inherited permissions from parent to child entities
- Fine-grained file and chat permissions
- Permission delegation and temporary access

### File System Allocation
- Organisation-level shared storage (configurable quota)
- Project-level dedicated storage
- Groups use parent organisation storage for media
- Storage quota management and monitoring

### Data Models
- Organisation entity with metadata and settings
- Group entity with chat focus
- Project entity with file system and task management
- Member roles and permission mappings

### Management Interfaces
- Organisation creation and management
- Group creation within organisations
- Project setup with file system initialization
- Member invitation and role management
- Permission configuration interfaces

## Deliverables
1. Three-tier hierarchy data models
2. Permission system implementation
3. File system allocation logic
4. Management interfaces for all entity types
5. Member and role management system

## Success Criteria
- Clear hierarchy with proper nesting
- Permissions work correctly across all levels
- File systems properly isolated and shared
- Intuitive management interfaces
- Scalable to hundreds of members per organisation

## Dependencies
- Contact management system (Task 2)
- File system implementation (Task 5)
- Authentication and authorization framework
EOF < /dev/null