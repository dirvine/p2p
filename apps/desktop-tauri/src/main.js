// Professional Saorsa - Main JavaScript
import { invoke } from '@tauri-apps/api/tauri';
import { appWindow } from '@tauri-apps/api/window';
import { WebRTCManager } from './webrtc.js';
import { CallUI } from './call-ui.js';
import { themeManager, ThemeSettings } from './theme.js';

// Global state
const state = {
    currentUser: null,
    currentSection: 'chat',
    currentChannel: null,
    currentTopic: null,
    currentProject: null,
    organizations: [],
    channels: [],
    contacts: [],
    messages: new Map(),
    topics: [],
    projects: [],
    unreadCounts: new Map(),
};

// Initialize the application
document.addEventListener('DOMContentLoaded', async () => {
    console.log('Initializing Professional Saorsa...');
    
    // Setup event listeners
    setupNavigationTabs();
    setupUserMenu();
    setupChatInterface();
    setupDiscussInterface();
    setupProjectsInterface();
    setupSearch();
    setupNotifications();
    
    // Load initial data
    await loadUserProfile();
    await loadOrganizations();
    await loadChannels();
    await loadContacts();
    
    // Connect to P2P network
    await connectToNetwork();
    
    // Setup real-time updates
    setupRealtimeUpdates();
});

// Navigation
function setupNavigationTabs() {
    const tabs = document.querySelectorAll('.nav-tab');
    tabs.forEach(tab => {
        tab.addEventListener('click', () => {
            const section = tab.dataset.section;
            switchSection(section);
        });
    });
}

function switchSection(section) {
    // Update tabs
    document.querySelectorAll('.nav-tab').forEach(tab => {
        tab.classList.toggle('active', tab.dataset.section === section);
    });
    
    // Update sections
    document.querySelectorAll('.content-section').forEach(sec => {
        sec.classList.toggle('active', sec.id === `${section}-section`);
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
}

// User Menu
function setupUserMenu() {
    const avatar = document.querySelector('.user-avatar');
    const dropdown = document.querySelector('.user-dropdown');
    
    avatar.addEventListener('click', (e) => {
        e.stopPropagation();
        dropdown.classList.toggle('hidden');
    });
    
    // Close dropdown when clicking outside
    document.addEventListener('click', () => {
        dropdown.classList.add('hidden');
    });
    
    // Setup dropdown items
    document.querySelectorAll('.dropdown-item').forEach(item => {
        item.addEventListener('click', handleUserMenuAction);
    });
}

async function handleUserMenuAction(e) {
    e.preventDefault();
    const action = e.target.textContent.trim();
    
    switch (action) {
        case 'Profile Settings':
            openProfileSettings();
            break;
        case 'Organization':
            openOrganizationSettings();
            break;
        case 'Teams':
            openTeamsManager();
            break;
        case 'Sign Out':
            await signOut();
            break;
    }
}

// Chat Interface
function setupChatInterface() {
    // Channel selection
    document.addEventListener('click', (e) => {
        if (e.target.closest('.channel-item')) {
            const channelItem = e.target.closest('.channel-item');
            selectChannel(channelItem);
        }
    });
    
    // Message composer
    const composer = document.querySelector('.composer-input');
    const sendBtn = document.querySelector('.send-button');
    
    composer?.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' && !e.shiftKey) {
            e.preventDefault();
            sendMessage();
        }
    });
    
    sendBtn?.addEventListener('click', sendMessage);
    
    // Voice/video calls
    document.querySelectorAll('.action-btn').forEach(btn => {
        btn.addEventListener('click', handleChatAction);
    });
}

async function handleChatAction(e) {
    const btn = e.currentTarget;
    const action = btn.getAttribute('title');
    
    if (!state.currentChannel) {
        showNotification('Please select a channel or contact first', 'warning');
        return;
    }
    
    // For DMs, get the user ID from the channel name
    const isDM = !state.currentChannel.startsWith('#');
    const targetUserId = isDM ? state.currentChannel : null;
    
    switch (action) {
        case 'Voice call':
            if (!targetUserId) {
                showNotification('Voice calls are only available in direct messages', 'info');
                return;
            }
            await startVoiceCall(targetUserId);
            break;
            
        case 'Video call':
            if (!targetUserId) {
                showNotification('Video calls are only available in direct messages', 'info');
                return;
            }
            await startVideoCall(targetUserId);
            break;
            
        case 'Channel info':
            showChannelInfo(state.currentChannel);
            break;
    }
}

async function startVoiceCall(userId) {
    try {
        await window.webRTCManager.startCall(userId, state.currentChannel, false);
        window.callUI.showActiveCall(userId, userId, false);
    } catch (error) {
        showNotification(`Failed to start voice call: ${error.message}`, 'error');
    }
}

async function startVideoCall(userId) {
    try {
        await window.webRTCManager.startCall(userId, state.currentChannel, true);
        window.callUI.showActiveCall(userId, userId, true);
    } catch (error) {
        showNotification(`Failed to start video call: ${error.message}`, 'error');
    }
}

function showChannelInfo(channel) {
    // Show channel info modal
    const modal = createModal({
        title: 'Channel Information',
        content: `
            <div class="channel-info-content">
                <h3>${channel}</h3>
                <p>Channel description and settings will be displayed here.</p>
                <div class="channel-stats">
                    <div class="stat">
                        <span class="stat-label">Members:</span>
                        <span class="stat-value">12</span>
                    </div>
                    <div class="stat">
                        <span class="stat-label">Created:</span>
                        <span class="stat-value">2 weeks ago</span>
                    </div>
                </div>
            </div>
        `,
        buttons: [
            {
                text: 'Close',
                onClick: () => modal.close()
            }
        ]
    });
}

function selectChannel(channelItem) {
    // Update UI
    document.querySelectorAll('.channel-item').forEach(item => {
        item.classList.remove('active');
    });
    channelItem.classList.add('active');
    
    // Get channel info
    const channelName = channelItem.querySelector('.channel-name').textContent;
    const isPrivate = channelItem.querySelector('.lock-icon') !== null;
    const isDM = channelItem.querySelector('.user-status') !== null;
    
    // Update header
    const chatTitle = document.querySelector('.chat-title');
    const chatSubtitle = document.querySelector('.chat-subtitle');
    
    if (isDM) {
        chatTitle.textContent = channelName;
        chatSubtitle.textContent = 'Direct Message';
    } else {
        chatTitle.textContent = `${isPrivate ? '🔒 ' : '# '}${channelName}`;
        chatSubtitle.textContent = getChannelDescription(channelName);
    }
    
    // Load messages
    loadChannelMessages(channelName);
    
    // Clear unread badge
    const unreadBadge = channelItem.querySelector('.unread-badge');
    if (unreadBadge) {
        unreadBadge.remove();
    }
    
    state.currentChannel = channelName;
}

async function sendMessage() {
    const composer = document.querySelector('.composer-input');
    const message = composer.value.trim();
    
    if (!message || !state.currentChannel) return;
    
    try {
        // Send message via Tauri
        await invoke('send_message', {
            channel: state.currentChannel,
            content: message,
            messageType: 'text'
        });
        
        // Clear composer
        composer.value = '';
        
        // Add message to UI immediately
        addMessageToUI({
            id: Date.now().toString(),
            author: state.currentUser.name,
            content: message,
            timestamp: new Date(),
            channel: state.currentChannel
        });
    } catch (error) {
        console.error('Failed to send message:', error);
        showNotification('Failed to send message', 'error');
    }
}

function addMessageToUI(message) {
    const messagesList = document.querySelector('.messages-list');
    const messageEl = createMessageElement(message);
    messagesList.appendChild(messageEl);
    
    // Scroll to bottom
    const messagesArea = document.querySelector('.messages-area');
    messagesArea.scrollTop = messagesArea.scrollHeight;
}

function createMessageElement(message) {
    const div = document.createElement('div');
    div.className = 'message';
    div.innerHTML = `
        <img src="${message.avatar || ''}" alt="${message.author}" class="message-avatar" />
        <div class="message-content">
            <div class="message-header">
                <span class="message-author">${message.author}</span>
                <span class="message-time">${formatTime(message.timestamp)}</span>
            </div>
            <div class="message-text">${escapeHtml(message.content)}</div>
        </div>
    `;
    return div;
}

// Discuss Interface
function setupDiscussInterface() {
    // Category selection
    document.addEventListener('click', (e) => {
        if (e.target.closest('.category-item')) {
            const categoryItem = e.target.closest('.category-item');
            selectCategory(categoryItem);
        }
    });
    
    // New topic button
    document.querySelector('.new-topic-btn')?.addEventListener('click', createNewTopic);
    
    // Topic interaction
    document.addEventListener('click', (e) => {
        if (e.target.closest('.topic-item')) {
            const topicItem = e.target.closest('.topic-item');
            openTopic(topicItem.dataset.topicId);
        }
    });
}

function selectCategory(categoryItem) {
    // Update UI
    document.querySelectorAll('.category-item').forEach(item => {
        item.classList.remove('active');
    });
    categoryItem.classList.add('active');
    
    const categoryName = categoryItem.querySelector('.category-name').textContent;
    
    // Update header
    document.querySelector('.topics-header h2').textContent = categoryName;
    
    // Load topics for category
    loadCategoryTopics(categoryName);
}

async function createNewTopic() {
    // Open topic creation modal
    const modal = createModal({
        title: 'Create New Topic',
        content: `
            <div class="form-group">
                <label>Title</label>
                <input type="text" id="topic-title" class="form-input" placeholder="Enter topic title">
            </div>
            <div class="form-group">
                <label>Category</label>
                <select id="topic-category" class="form-select">
                    <option>General Discussion</option>
                    <option>Announcements</option>
                    <option>Knowledge Base</option>
                </select>
            </div>
            <div class="form-group">
                <label>Content</label>
                <textarea id="topic-content" class="form-textarea" rows="10" placeholder="Write your topic content..."></textarea>
            </div>
            <div class="form-group">
                <label>Tags</label>
                <input type="text" id="topic-tags" class="form-input" placeholder="Enter tags separated by commas">
            </div>
        `,
        buttons: [
            {
                text: 'Cancel',
                class: 'secondary',
                onClick: () => modal.close()
            },
            {
                text: 'Create Topic',
                class: 'primary',
                onClick: async () => {
                    const title = document.getElementById('topic-title').value;
                    const category = document.getElementById('topic-category').value;
                    const content = document.getElementById('topic-content').value;
                    const tags = document.getElementById('topic-tags').value.split(',').map(t => t.trim());
                    
                    await createTopic({ title, category, content, tags });
                    modal.close();
                }
            }
        ]
    });
}

// Projects Interface
function setupProjectsInterface() {
    // Tree navigation
    document.addEventListener('click', (e) => {
        if (e.target.closest('.tree-node')) {
            const treeItem = e.target.closest('.tree-item');
            toggleTreeItem(treeItem);
            
            const isFile = !treeItem.querySelector('.tree-children');
            if (isFile) {
                selectProjectItem(treeItem);
            }
        }
    });
    
    // File operations
    document.addEventListener('click', (e) => {
        if (e.target.closest('.file-card')) {
            const fileCard = e.target.closest('.file-card');
            openFile(fileCard.dataset.fileId);
        }
    });
    
    // Upload button
    document.querySelector('.project-actions .primary')?.addEventListener('click', uploadFiles);
}

function toggleTreeItem(treeItem) {
    treeItem.classList.toggle('expanded');
}

function selectProjectItem(treeItem) {
    const itemName = treeItem.querySelector('.tree-label').textContent;
    updateBreadcrumb(itemName);
    loadProjectFiles(itemName);
}

async function uploadFiles() {
    // Open file picker
    const input = document.createElement('input');
    input.type = 'file';
    input.multiple = true;
    
    input.onchange = async (e) => {
        const files = Array.from(e.target.files);
        
        for (const file of files) {
            await uploadFile(file);
        }
    };
    
    input.click();
}

async function uploadFile(file) {
    try {
        // Show upload progress
        const progressModal = showProgress(`Uploading ${file.name}...`);
        
        // Convert file to base64 for Tauri
        const reader = new FileReader();
        reader.onload = async (e) => {
            const content = e.target.result.split(',')[1]; // Remove data URL prefix
            
            await invoke('upload_file', {
                projectId: state.currentProject,
                fileName: file.name,
                content: content,
                mimeType: file.type
            });
            
            progressModal.close();
            showNotification(`${file.name} uploaded successfully`, 'success');
            
            // Refresh file list
            loadProjectFiles(state.currentProject);
        };
        
        reader.readAsDataURL(file);
    } catch (error) {
        console.error('Failed to upload file:', error);
        showNotification(`Failed to upload ${file.name}`, 'error');
    }
}

// Search functionality
function setupSearch() {
    const searchBtn = document.querySelector('.nav-action[title="Search"]');
    searchBtn?.addEventListener('click', openGlobalSearch);
}

function openGlobalSearch() {
    const modal = createModal({
        title: 'Search',
        class: 'search-modal',
        content: `
            <div class="search-container">
                <input type="text" id="global-search" class="search-input" placeholder="Search messages, topics, files..." autofocus>
                <div class="search-filters">
                    <label><input type="checkbox" checked> Messages</label>
                    <label><input type="checkbox" checked> Topics</label>
                    <label><input type="checkbox" checked> Files</label>
                </div>
                <div id="search-results" class="search-results">
                    <!-- Results will appear here -->
                </div>
            </div>
        `,
        buttons: [
            {
                text: 'Close',
                onClick: () => modal.close()
            }
        ]
    });
    
    // Setup search input
    const searchInput = document.getElementById('global-search');
    let searchTimeout;
    
    searchInput.addEventListener('input', (e) => {
        clearTimeout(searchTimeout);
        searchTimeout = setTimeout(() => {
            performSearch(e.target.value);
        }, 300);
    });
}

async function performSearch(query) {
    if (!query.trim()) {
        document.getElementById('search-results').innerHTML = '';
        return;
    }
    
    try {
        const results = await invoke('search', { query });
        displaySearchResults(results);
    } catch (error) {
        console.error('Search failed:', error);
    }
}

// Notifications
function setupNotifications() {
    const notificationBtn = document.querySelector('.nav-action[title="Notifications"]');
    notificationBtn?.addEventListener('click', openNotificationCenter);
    
    // Listen for new notifications
    window.addEventListener('new-notification', (e) => {
        updateNotificationBadge(e.detail.count);
    });
}

function updateNotificationBadge(count) {
    const badge = document.querySelector('.notification-badge');
    if (count > 0) {
        badge.textContent = count > 99 ? '99+' : count;
        badge.style.display = 'flex';
    } else {
        badge.style.display = 'none';
    }
}

// Real-time updates
function setupRealtimeUpdates() {
    // Listen for various events from Tauri backend
    window.__TAURI__.event.listen('new-message', (event) => {
        handleNewMessage(event.payload);
    });
    
    window.__TAURI__.event.listen('user-status-change', (event) => {
        updateUserStatus(event.payload);
    });
    
    window.__TAURI__.event.listen('new-topic', (event) => {
        handleNewTopic(event.payload);
    });
    
    window.__TAURI__.event.listen('file-uploaded', (event) => {
        handleFileUploaded(event.payload);
    });
    
    // WebRTC call events
    window.__TAURI__.event.listen('incoming-call', (event) => {
        handleIncomingCall(event.payload);
    });
    
    window.__TAURI__.event.listen('call-answer', (event) => {
        handleCallAnswer(event.payload);
    });
    
    window.__TAURI__.event.listen('call-ended', (event) => {
        handleCallEnded(event.payload);
    });
    
    window.__TAURI__.event.listen('ice-candidate', (event) => {
        handleIceCandidate(event.payload);
    });
}

// WebRTC call handlers
async function handleIncomingCall(data) {
    const { userId, channelId, offer, isVideo } = data;
    
    // Get user name from contacts
    const user = state.contacts.find(c => c.id === userId);
    const userName = user?.name || userId;
    
    // Handle the incoming call
    await window.webRTCManager.handleIncomingCall(userId, channelId, offer, isVideo);
    
    // Show incoming call UI
    window.callUI.showIncomingCall(userId, userName, channelId, isVideo);
}

async function handleCallAnswer(data) {
    const { userId, answer } = data;
    const call = window.webRTCManager.calls.get(userId);
    
    if (call) {
        await call.setRemoteAnswer(answer);
    }
}

async function handleCallEnded(data) {
    const { userId, reason } = data;
    await window.webRTCManager.endCall(userId, reason);
}

async function handleIceCandidate(data) {
    const { userId, candidate } = data;
    const call = window.webRTCManager.calls.get(userId);
    
    if (call) {
        await call.addIceCandidate(candidate);
        
        // Process queued candidates if needed
        if (call.pc.remoteDescription && call.iceCandidates.length > 0) {
            await call.processQueuedCandidates();
        }
    }
}

// Utility functions
function formatTime(timestamp) {
    const date = new Date(timestamp);
    const now = new Date();
    const diff = now - date;
    
    if (diff < 60000) {
        return 'just now';
    } else if (diff < 3600000) {
        return `${Math.floor(diff / 60000)} minutes ago`;
    } else if (diff < 86400000) {
        return `${Math.floor(diff / 3600000)} hours ago`;
    } else {
        return date.toLocaleDateString();
    }
}

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

function createModal({ title, content, buttons, class: className }) {
    const modal = document.createElement('div');
    modal.className = `modal ${className || ''}`;
    
    modal.innerHTML = `
        <div class="modal-backdrop"></div>
        <div class="modal-content">
            <div class="modal-header">
                <h3>${title}</h3>
                <button class="close-btn">&times;</button>
            </div>
            <div class="modal-body">
                ${content}
            </div>
            <div class="modal-footer">
                ${buttons.map(btn => `
                    <button class="btn btn-${btn.class || 'default'}">${btn.text}</button>
                `).join('')}
            </div>
        </div>
    `;
    
    document.body.appendChild(modal);
    
    // Setup event listeners
    modal.querySelector('.close-btn').addEventListener('click', () => modal.remove());
    modal.querySelector('.modal-backdrop').addEventListener('click', () => modal.remove());
    
    buttons.forEach((btn, index) => {
        const btnEl = modal.querySelectorAll('.modal-footer button')[index];
        btnEl.addEventListener('click', btn.onClick);
    });
    
    return {
        close: () => modal.remove(),
        element: modal
    };
}

function showNotification(message, type = 'info') {
    const notification = document.createElement('div');
    notification.className = `notification notification-${type}`;
    notification.textContent = message;
    
    document.body.appendChild(notification);
    
    setTimeout(() => {
        notification.classList.add('show');
    }, 10);
    
    setTimeout(() => {
        notification.classList.remove('show');
        setTimeout(() => notification.remove(), 300);
    }, 3000);
}

function showProgress(message) {
    const modal = createModal({
        title: 'Progress',
        content: `
            <div class="progress-container">
                <p>${message}</p>
                <div class="progress-bar">
                    <div class="progress-fill"></div>
                </div>
            </div>
        `,
        buttons: []
    });
    
    return modal;
}

// Load initial data
async function loadUserProfile() {
    try {
        const profile = await invoke('get_user_profile');
        state.currentUser = profile;
        
        // Update UI
        updateUserDisplay(profile);
    } catch (error) {
        console.error('Failed to load user profile:', error);
    }
}

function updateUserDisplay(profile) {
    // Update avatar
    const avatarPlaceholder = document.querySelector('.avatar-placeholder');
    if (profile.avatar) {
        const img = document.getElementById('user-avatar-img');
        img.src = profile.avatar;
        img.style.display = 'block';
        avatarPlaceholder.style.display = 'none';
    } else {
        avatarPlaceholder.textContent = getInitials(profile.name);
    }
    
    // Update user info
    document.querySelector('.user-name').textContent = profile.name;
    document.querySelector('.user-role').textContent = profile.role || 'Team Member';
}

function getInitials(name) {
    return name
        .split(' ')
        .map(n => n[0])
        .join('')
        .toUpperCase()
        .slice(0, 2);
}

async function loadOrganizations() {
    try {
        const orgs = await invoke('get_organizations');
        state.organizations = orgs;
        
        // Update org selector
        const orgDropdown = document.querySelector('.org-dropdown');
        orgDropdown.innerHTML = orgs.map(org => 
            `<option value="${org.id}">${org.name}</option>`
        ).join('');
    } catch (error) {
        console.error('Failed to load organizations:', error);
    }
}

async function loadChannels() {
    try {
        const channels = await invoke('get_channels');
        state.channels = channels;
        
        // Update channel list
        updateChannelList(channels);
    } catch (error) {
        console.error('Failed to load channels:', error);
    }
}

function updateChannelList(channels) {
    const publicChannels = channels.filter(c => c.type === 'public');
    const privateChannels = channels.filter(c => c.type === 'private');
    
    // Update public channels
    const publicSection = document.querySelector('.channel-section:first-child .channel-items');
    publicSection.innerHTML = publicChannels.map(channel => `
        <div class="channel-item" data-channel-id="${channel.id}">
            <span class="channel-prefix">#</span>
            <span class="channel-name">${channel.name}</span>
            ${channel.unread > 0 ? `<span class="unread-badge">${channel.unread}</span>` : ''}
        </div>
    `).join('');
    
    // Add private channels
    privateChannels.forEach(channel => {
        const item = document.createElement('div');
        item.className = 'channel-item';
        item.dataset.channelId = channel.id;
        item.innerHTML = `
            <svg class="channel-prefix lock-icon" width="12" height="12" viewBox="0 0 12 12">
                <rect x="3" y="5" width="6" height="5" rx="1" fill="currentColor"/>
                <path d="M4 5V3a2 2 0 114 0v2" stroke="currentColor" fill="none"/>
            </svg>
            <span class="channel-name">${channel.name}</span>
            ${channel.unread > 0 ? `<span class="unread-badge">${channel.unread}</span>` : ''}
        `;
        publicSection.appendChild(item);
    });
}

async function loadContacts() {
    try {
        const contacts = await invoke('get_contacts');
        state.contacts = contacts;
        
        // Update DM list
        updateContactsList(contacts);
    } catch (error) {
        console.error('Failed to load contacts:', error);
    }
}

function updateContactsList(contacts) {
    const dmSection = document.querySelector('.channel-section:last-child .channel-items');
    dmSection.innerHTML = contacts.map(contact => `
        <div class="channel-item" data-contact-id="${contact.id}">
            <div class="user-status ${contact.status}"></div>
            <span class="channel-name">${contact.name}</span>
            ${contact.unread > 0 ? `<span class="unread-badge">${contact.unread}</span>` : ''}
        </div>
    `).join('');
}

async function connectToNetwork() {
    try {
        await invoke('connect_to_network');
        console.log('Connected to P2P network');
    } catch (error) {
        console.error('Failed to connect to network:', error);
        showNotification('Failed to connect to network', 'error');
    }
}

// Helper functions for getting channel descriptions
function getChannelDescription(channelName) {
    const descriptions = {
        'general': 'Company-wide announcements and general discussion',
        'engineering': 'Engineering team discussions and updates',
        'leadership': 'Leadership team private discussions',
        'random': 'Non-work banter and random discussions'
    };
    
    return descriptions[channelName] || 'Channel for team collaboration';
}

// Settings Functions
function openProfileSettings() {
    const themeSettings = new ThemeSettings(themeManager);
    
    const modal = document.createElement('div');
    modal.className = 'settings-modal';
    modal.innerHTML = `
        <div class="settings-content">
            <div class="settings-header">
                <h2 class="settings-title">Settings</h2>
                <button class="close-btn">&times;</button>
            </div>
            <div class="settings-body">
                <div class="settings-nav">
                    <div class="settings-nav-item active" data-section="appearance">
                        <svg width="20" height="20" viewBox="0 0 20 20">
                            <path d="M10 2a6 6 0 00-6 6v3.586l-.707.707A1 1 0 004 14h12a1 1 0 00.707-1.707L16 11.586V8a6 6 0 00-6-6z" fill="currentColor"/>
                        </svg>
                        <span>Appearance</span>
                    </div>
                    <div class="settings-nav-item" data-section="profile">
                        <svg width="20" height="20" viewBox="0 0 20 20">
                            <path d="M10 9a3 3 0 100-6 3 3 0 000 6z" fill="currentColor"/>
                            <path d="M6 15a4 4 0 118 0v1H6v-1z" fill="currentColor"/>
                        </svg>
                        <span>Profile</span>
                    </div>
                    <div class="settings-nav-item" data-section="privacy">
                        <svg width="20" height="20" viewBox="0 0 20 20">
                            <path d="M10 2a5 5 0 00-5 5v2a2 2 0 00-2 2v5a2 2 0 002 2h10a2 2 0 002-2v-5a2 2 0 00-2-2V7a5 5 0 00-5-5zm3 5v2H7V7a3 3 0 016 0z" fill="currentColor"/>
                        </svg>
                        <span>Privacy & Security</span>
                    </div>
                    <div class="settings-nav-item" data-section="notifications">
                        <svg width="20" height="20" viewBox="0 0 20 20">
                            <path d="M10 2a6 6 0 00-6 6v3.586l-.707.707A1 1 0 004 14h12a1 1 0 00.707-1.707L16 11.586V8a6 6 0 00-6-6z" fill="currentColor"/>
                            <path d="M10 18a3 3 0 01-3-3h6a3 3 0 01-3 3z" fill="currentColor"/>
                        </svg>
                        <span>Notifications</span>
                    </div>
                </div>
                <div class="settings-content-area">
                    ${themeSettings.render()}
                </div>
            </div>
        </div>
    `;
    
    document.body.appendChild(modal);
    
    // Attach theme settings event listeners
    themeSettings.attachEventListeners(modal);
    
    // Close button
    modal.querySelector('.close-btn').addEventListener('click', () => {
        modal.remove();
    });
    
    // Close on backdrop click
    modal.addEventListener('click', (e) => {
        if (e.target === modal) {
            modal.remove();
        }
    });
    
    // Navigation
    const navItems = modal.querySelectorAll('.settings-nav-item');
    navItems.forEach(item => {
        item.addEventListener('click', () => {
            navItems.forEach(nav => nav.classList.remove('active'));
            item.classList.add('active');
            
            const section = item.dataset.section;
            const contentArea = modal.querySelector('.settings-content-area');
            
            switch (section) {
                case 'appearance':
                    contentArea.innerHTML = themeSettings.render();
                    themeSettings.attachEventListeners(modal);
                    break;
                case 'profile':
                    contentArea.innerHTML = renderProfileSettings();
                    break;
                case 'privacy':
                    contentArea.innerHTML = renderPrivacySettings();
                    break;
                case 'notifications':
                    contentArea.innerHTML = renderNotificationSettings();
                    break;
            }
        });
    });
}

function renderProfileSettings() {
    return `
        <div class="settings-section">
            <h3 class="settings-title">Profile Information</h3>
            <div class="settings-group">
                <label class="settings-label">Display Name</label>
                <input type="text" class="form-input" value="${state.currentUser?.name || ''}" />
            </div>
            <div class="settings-group">
                <label class="settings-label">Status Message</label>
                <input type="text" class="form-input" placeholder="What's your status?" />
            </div>
            <div class="settings-group">
                <label class="settings-label">Avatar</label>
                <button class="btn btn-secondary">Change Avatar</button>
            </div>
        </div>
    `;
}

function renderPrivacySettings() {
    return `
        <div class="settings-section">
            <h3 class="settings-title">Privacy & Security</h3>
            <div class="settings-group">
                <label class="settings-label">Who can message you</label>
                <select class="form-select">
                    <option>Everyone</option>
                    <option>Contacts only</option>
                    <option>Nobody</option>
                </select>
            </div>
            <div class="settings-group">
                <label class="settings-label">
                    <input type="checkbox" checked />
                    Show online status
                </label>
            </div>
            <div class="settings-group">
                <label class="settings-label">
                    <input type="checkbox" checked />
                    Show last seen
                </label>
            </div>
        </div>
    `;
}

function renderNotificationSettings() {
    return `
        <div class="settings-section">
            <h3 class="settings-title">Notification Preferences</h3>
            <div class="settings-group">
                <label class="settings-label">
                    <input type="checkbox" checked />
                    Desktop notifications
                </label>
            </div>
            <div class="settings-group">
                <label class="settings-label">
                    <input type="checkbox" checked />
                    Message sounds
                </label>
            </div>
            <div class="settings-group">
                <label class="settings-label">
                    <input type="checkbox" checked />
                    Show message preview
                </label>
            </div>
        </div>
    `;
}

function openOrganizationSettings() {
    // Implementation for organization settings
    showNotification('Organization settings coming soon', 'info');
}

function openTeamsManager() {
    // Implementation for teams manager
    showNotification('Teams manager coming soon', 'info');
}

async function signOut() {
    // Implementation for sign out
    showNotification('Sign out functionality coming soon', 'info');
}

// Initialize WebRTC
window.webRTCManager = new WebRTCManager();
window.callUI = new CallUI();

// Export for use in other modules
window.SaorsaPro = {
    state,
    showNotification,
    createModal,
    invoke
};