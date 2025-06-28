// Saorsa Professional - Simple Working JavaScript
console.log('Loading Saorsa Professional...');

// Global state
const state = {
    currentUser: {
        name: 'Demo User',
        role: 'Team Leader',
        avatar: null,
        id: 'demo-user-123'
    },
    currentSection: 'chat',
    currentChannel: 'general',
    currentTopic: null,
    currentProject: null,
    organizations: [
        { id: 'demo-corp', name: 'Acme Corporation' },
        { id: 'personal', name: 'Personal Workspace' }
    ],
    channels: [
        { id: 'general', name: 'general', type: 'public', unread: 2 },
        { id: 'engineering', name: 'engineering', type: 'public', unread: 0 },
        { id: 'leadership', name: 'leadership', type: 'private', unread: 0 }
    ],
    contacts: [
        { id: '1', name: 'Alice Johnson', status: 'online' },
        { id: '2', name: 'Bob Smith', status: 'away', unread: 5 },
        { id: '3', name: 'Carol Williams', status: 'offline' }
    ]
};

// Initialize the application
document.addEventListener('DOMContentLoaded', function() {
    console.log('Initializing Saorsa Professional...');
    
    // Setup event listeners
    setupNavigationTabs();
    setupUserMenu();
    
    // Load initial data
    loadInitialData();
    
    console.log('Saorsa Professional initialized successfully');
});

// Navigation
function setupNavigationTabs() {
    console.log('Setting up navigation tabs...');
    const tabs = document.querySelectorAll('.nav-tab');
    console.log('Found', tabs.length, 'navigation tabs');
    
    tabs.forEach(function(tab, index) {
        console.log('Setting up tab', index, 'with section:', tab.dataset.section);
        tab.addEventListener('click', function(e) {
            e.preventDefault();
            console.log('Tab clicked:', tab.dataset.section);
            const section = tab.dataset.section;
            switchSection(section);
        });
    });
}

function switchSection(section) {
    console.log('Switching to section:', section);
    
    // Update tabs
    document.querySelectorAll('.nav-tab').forEach(function(tab) {
        if (tab.dataset.section === section) {
            tab.classList.add('active');
        } else {
            tab.classList.remove('active');
        }
    });
    
    // Update sections
    document.querySelectorAll('.content-section').forEach(function(sec) {
        if (sec.id === section + '-section') {
            sec.classList.add('active');
        } else {
            sec.classList.remove('active');
        }
    });
    
    state.currentSection = section;
    
    // Load section-specific data
    switch (section) {
        case 'chat':
            loadChatData();
            break;
        case 'discuss':
            loadDiscussData();
            break;
        case 'projects':
            loadProjectsData();
            break;
    }
    
    console.log('Section switched to:', section);
}

// User Menu
function setupUserMenu() {
    const avatar = document.querySelector('.user-avatar');
    const dropdown = document.querySelector('.user-dropdown');
    
    if (avatar && dropdown) {
        avatar.addEventListener('click', function(e) {
            e.stopPropagation();
            dropdown.classList.toggle('hidden');
        });
        
        // Close dropdown when clicking outside
        document.addEventListener('click', function() {
            dropdown.classList.add('hidden');
        });
    }
}

// Load initial data
function loadInitialData() {
    console.log('Loading initial data...');
    
    // Update user display
    updateUserDisplay(state.currentUser);
    
    // Update organizations
    updateOrganizations(state.organizations);
    
    // Update channels
    updateChannelList(state.channels);
    
    // Update contacts
    updateContactsList(state.contacts);
    
    console.log('Initial data loaded');
}

function updateUserDisplay(profile) {
    // Update avatar
    const avatarPlaceholder = document.querySelector('.avatar-placeholder');
    if (avatarPlaceholder) {
        avatarPlaceholder.textContent = getInitials(profile.name);
    }
    
    // Update user info
    const userName = document.querySelector('.user-name');
    const userRole = document.querySelector('.user-role');
    
    if (userName) userName.textContent = profile.name;
    if (userRole) userRole.textContent = profile.role || 'Team Member';
}

function getInitials(name) {
    return name
        .split(' ')
        .map(function(n) { return n[0]; })
        .join('')
        .toUpperCase()
        .slice(0, 2);
}

function updateOrganizations(orgs) {
    const orgDropdown = document.querySelector('.org-dropdown');
    if (orgDropdown) {
        orgDropdown.innerHTML = orgs.map(function(org) {
            return '<option value="' + org.id + '">' + org.name + '</option>';
        }).join('');
    }
}

function updateChannelList(channels) {
    const publicChannels = channels.filter(function(c) { return c.type === 'public'; });
    const privateChannels = channels.filter(function(c) { return c.type === 'private'; });
    
    // Update public channels
    const channelItems = document.querySelector('.channel-items');
    if (channelItems) {
        channelItems.innerHTML = publicChannels.map(function(channel) {
            const activeClass = channel.id === state.currentChannel ? ' active' : '';
            const unreadBadge = channel.unread > 0 ? 
                '<span class="unread-badge">' + channel.unread + '</span>' : '';
            
            return '<div class="channel-item' + activeClass + '" data-channel="' + channel.id + '">' +
                '<span class="channel-prefix">#</span>' +
                '<span class="channel-name">' + channel.name + '</span>' +
                unreadBadge +
                '</div>';
        }).join('');
        
        // Add click handlers for channels
        channelItems.querySelectorAll('.channel-item').forEach(function(item) {
            item.addEventListener('click', function() {
                selectChannel(item.dataset.channel);
            });
        });
    }
}

function updateContactsList(contacts) {
    // This would update the DM list in the chat section
    console.log('Contacts loaded:', contacts.length);
}

function selectChannel(channelId) {
    console.log('Channel selected:', channelId);
    state.currentChannel = channelId;
    
    // Update channel header
    const chatTitle = document.getElementById('current-contact-name') || document.querySelector('.chat-title');
    if (chatTitle) {
        chatTitle.textContent = '#' + channelId;
    }
    
    // Update active channel in UI
    document.querySelectorAll('.channel-item').forEach(function(item) {
        if (item.dataset.channel === channelId) {
            item.classList.add('active');
        } else {
            item.classList.remove('active');
        }
    });
}

// Section-specific data loading
function loadChatData() {
    console.log('Loading chat data...');
    // This would load messages for the current channel
}

function loadDiscussData() {
    console.log('Loading discussion data...');
    // This would load forum topics
}

function loadProjectsData() {
    console.log('Loading projects data...');
    // This would load project files and info
}

// Utility functions
function showNotification(message, type) {
    type = type || 'info';
    console.log('Notification (' + type + '):', message);
    
    const notification = document.createElement('div');
    notification.className = 'notification notification-' + type;
    notification.textContent = message;
    
    document.body.appendChild(notification);
    
    setTimeout(function() {
        notification.classList.add('show');
    }, 10);
    
    setTimeout(function() {
        notification.classList.remove('show');
        setTimeout(function() {
            notification.remove();
        }, 300);
    }, 3000);
}

console.log('Saorsa Professional JavaScript loaded');