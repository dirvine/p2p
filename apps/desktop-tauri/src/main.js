const { invoke } = window.__TAURI__.core;

// Application state
let appState = {
  networkInitialized: false,
  currentContact: null,
  contacts: new Map(),
  messages: new Map(),
  layoutPosition: 'left', // left, right, top, bottom
  systemMenuOpen: false,
  userProfile: null,
  userIdentity: null,
  contactRequests: new Map(),
  profileModalOpen: false,
  contactModalOpen: false
};

// Initialize the application
window.addEventListener('DOMContentLoaded', async () => {
  console.log('Saorsa starting...');
  
  initializeEventListeners();
  await initializeNetwork();
  loadContacts();
  loadSettings();
});

// Initialize all event listeners
function initializeEventListeners() {
  // Layout positioning
  document.getElementById('layout-btn').addEventListener('click', openLayoutModal);
  document.getElementById('close-layout-modal').addEventListener('click', closeLayoutModal);
  
  // Layout option buttons
  document.querySelectorAll('.layout-option').forEach(btn => {
    btn.addEventListener('click', (e) => changeLayout(e.target.dataset.position));
  });
  
  // Add contact dialog
  document.getElementById('add-contact-btn').addEventListener('click', openConnectDialog);
  document.getElementById('close-connect-dialog').addEventListener('click', closeConnectDialog);
  document.getElementById('cancel-connect-btn').addEventListener('click', closeConnectDialog);
  document.getElementById('connect-btn').addEventListener('click', connectToPeer);
  
  // Message input
  const messageInput = document.getElementById('message-input');
  const sendBtn = document.getElementById('send-btn');
  
  messageInput.addEventListener('keypress', (e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  });
  
  sendBtn.addEventListener('click', sendMessage);
  
  // System menu
  document.getElementById('system-menu').addEventListener('click', (e) => {
    if (e.target.id === 'system-menu') {
      closeSystemMenu(); // Click outside to close
    }
  });
  
  document.getElementById('close-menu-btn').addEventListener('click', closeSystemMenu);
  
  // System menu options
  document.querySelectorAll('.menu-option').forEach(btn => {
    btn.addEventListener('click', (e) => executeSystemCommand(e.target.dataset.command));
  });
  
  // Modal click outside to close
  document.getElementById('layout-modal').addEventListener('click', (e) => {
    if (e.target.id === 'layout-modal') closeLayoutModal();
  });
  
  document.getElementById('connect-dialog').addEventListener('click', (e) => {
    if (e.target.id === 'connect-dialog') closeConnectDialog();
  });

  // Profile modal event listeners
  document.getElementById('close-profile-modal').addEventListener('click', closeProfileModal);
  document.getElementById('cancel-profile-btn').addEventListener('click', closeProfileModal);
  document.getElementById('save-profile-btn').addEventListener('click', saveProfile);
  
  // Profile tab switching
  document.querySelectorAll('#profile-modal .tab-btn').forEach(btn => {
    btn.addEventListener('click', (e) => switchProfileTab(e.target.dataset.tab));
  });
  
  // Profile actions
  document.getElementById('upload-avatar-btn').addEventListener('click', () => {
    document.getElementById('avatar-upload').click();
  });
  document.getElementById('avatar-upload').addEventListener('change', handleAvatarUpload);
  document.getElementById('remove-avatar-btn').addEventListener('click', removeAvatar);
  document.getElementById('copy-user-id').addEventListener('click', () => copyToClipboard('user-id'));
  document.getElementById('copy-public-key').addEventListener('click', () => copyToClipboard('public-key'));
  document.getElementById('create-new-identity-btn').addEventListener('click', createNewIdentity);
  document.getElementById('export-identity-btn').addEventListener('click', exportIdentity);
  document.getElementById('import-identity-btn').addEventListener('click', () => {
    document.getElementById('identity-file-input').click();
  });
  document.getElementById('identity-file-input').addEventListener('change', importIdentity);
  document.getElementById('bind-ipv6-btn').addEventListener('click', bindIPv6Identity);

  // Contact modal event listeners
  document.getElementById('close-contact-modal').addEventListener('click', closeContactModal);
  
  // Contact tab switching
  document.querySelectorAll('#contact-modal .tab-btn').forEach(btn => {
    btn.addEventListener('click', (e) => switchContactTab(e.target.dataset.tab));
  });
  
  // Contact actions
  document.getElementById('refresh-contacts-btn').addEventListener('click', refreshContacts);
  document.getElementById('search-btn').addEventListener('click', searchContacts);
  document.getElementById('search-query').addEventListener('keypress', (e) => {
    if (e.key === 'Enter') searchContacts();
  });

  // Profile modal click outside to close
  document.getElementById('profile-modal').addEventListener('click', (e) => {
    if (e.target.id === 'profile-modal') closeProfileModal();
  });
  
  // Contact modal click outside to close
  document.getElementById('contact-modal').addEventListener('click', (e) => {
    if (e.target.id === 'contact-modal') closeContactModal();
  });
}

// Initialize P2P network
async function initializeNetwork() {
  try {
    updateNetworkStatus('Connecting...');
    
    const result = await invoke('init_network', {
      listenPort: 9000,
      bootstrapNodes: [
        '/ip4/127.0.0.1/tcp/9001',
        '/ip4/127.0.0.1/tcp/9002'
      ]
    });
    
    console.log('Network initialized:', result);
    appState.networkInitialized = true;
    updateNetworkStatus('Online');
    
    // Enable message input
    document.getElementById('message-input').disabled = false;
    document.getElementById('send-btn').disabled = false;
    
  } catch (error) {
    console.error('Failed to initialize network:', error);
    updateNetworkStatus('Failed to connect');
  }
}

// Update network status indicator
function updateNetworkStatus(status) {
  const statusElement = document.getElementById('network-status');
  
  if (status === 'Online') {
    statusElement.textContent = 'Online';
    statusElement.className = 'status-online';
  } else {
    statusElement.textContent = status;
    statusElement.className = 'status-offline';
  }
}

// Load contacts from the backend
async function loadContacts() {
  try {
    const contacts = await invoke('get_contacts');
    
    appState.contacts.clear();
    contacts.forEach(contact => {
      appState.contacts.set(contact.id, contact);
    });
    
    renderContacts();
    
  } catch (error) {
    console.error('Failed to load contacts:', error);
  }
}

// Render contacts list
function renderContacts() {
  const contactsList = document.getElementById('contacts-list');
  contactsList.innerHTML = '';
  
  appState.contacts.forEach(contact => {
    const contactElement = createContactElement(contact);
    contactsList.appendChild(contactElement);
  });
}

// Create contact list item element
function createContactElement(contact) {
  const contactDiv = document.createElement('div');
  contactDiv.className = `contact-item ${contact.id === 'system' ? 'system' : ''} ${contact.is_blocked ? 'blocked' : ''}`;
  contactDiv.dataset.contactId = contact.id;
  
  // Generate avatar content
  const avatarContent = contact.id === 'system' ? '🤖' : 
    (contact.nickname ? contact.nickname.charAt(0) : contact.name.charAt(0)).toUpperCase();
  
  // Display name with nickname
  const displayName = contact.nickname ? 
    `${contact.nickname} (${contact.name})` : contact.name;
  
  contactDiv.innerHTML = `
    <div class="contact-avatar">${avatarContent}</div>
    <div class="contact-info">
      <div class="contact-name">${displayName}</div>
      <div class="contact-address">${contact.three_word_address}</div>
      ${contact.category ? `<div class="contact-category">${contact.category}</div>` : ''}
    </div>
    <div class="contact-status">
      ${contact.is_blocked ? '<span class="blocked-indicator" title="Blocked">🚫</span>' : ''}
      ${contact.is_online && !contact.is_blocked ? '<div class="online-indicator"></div>' : ''}
      ${contact.unread_count > 0 && !contact.is_blocked ? `<div class="contact-unread">${contact.unread_count}</div>` : ''}
    </div>
  `;
  
  // Left click to select
  contactDiv.addEventListener('click', () => {
    if (!contact.is_blocked) {
      selectContact(contact.id);
    } else {
      showNotification('This contact is blocked', 'warning');
    }
  });
  
  // Right click for context menu
  if (contact.id !== 'system') {
    contactDiv.addEventListener('contextmenu', (e) => {
      e.preventDefault();
      showContactContextMenu(e, contact);
    });
  }
  
  return contactDiv;
}

// Select a contact for chat
async function selectContact(contactId) {
  // Remove previous active state
  document.querySelectorAll('.contact-item').forEach(item => {
    item.classList.remove('active');
  });
  
  // Set new active state
  const contactElement = document.querySelector(`[data-contact-id="${contactId}"]`);
  if (contactElement) {
    contactElement.classList.add('active');
  }
  
  appState.currentContact = contactId;
  const contact = appState.contacts.get(contactId);
  
  // Update chat header
  document.getElementById('current-contact-name').textContent = contact.name;
  document.getElementById('current-contact-status').textContent = 
    contact.is_online ? 'Online' : `Last seen ${new Date(contact.last_seen * 1000).toLocaleString()}`;
  
  // Show system menu if system contact is selected
  if (contactId === 'system') {
    openSystemMenu();
  } else {
    closeSystemMenu();
  }
  
  // Load messages
  await loadMessages(contactId);
}

// Load messages for a contact
async function loadMessages(contactId) {
  try {
    const messages = await invoke('get_messages', { contactId });
    
    appState.messages.set(contactId, messages);
    renderMessages(messages);
    
  } catch (error) {
    console.error('Failed to load messages:', error);
  }
}

// Render messages in the chat area
function renderMessages(messages) {
  const messagesList = document.getElementById('messages-list');
  messagesList.innerHTML = '';
  
  messages.forEach(message => {
    const messageElement = createMessageElement(message);
    messagesList.appendChild(messageElement);
  });
  
  // Scroll to bottom
  messagesList.scrollTop = messagesList.scrollHeight;
}

// Create message element
function createMessageElement(message) {
  const messageDiv = document.createElement('div');
  const messageClass = message.is_from_me ? 'sent' : 
    (message.from_peer === 'system' ? 'system' : 'received');
  
  messageDiv.className = `message ${messageClass}`;
  messageDiv.innerHTML = `
    <div class="message-content">${escapeHtml(message.content)}</div>
    <div class="message-time">${new Date(message.timestamp * 1000).toLocaleTimeString()}</div>
  `;
  
  return messageDiv;
}

// Send a message
async function sendMessage() {
  const messageInput = document.getElementById('message-input');
  const content = messageInput.value.trim();
  
  if (!content || !appState.currentContact) {
    return;
  }
  
  // Check if contact is blocked
  const contact = appState.contacts.get(appState.currentContact);
  if (contact && contact.is_blocked) {
    showNotification('Cannot send messages to blocked contacts', 'error');
    return;
  }
  
  try {
    await invoke('send_message', {
      contactId: appState.currentContact,
      content: content
    });
    
    messageInput.value = '';
    
    // Reload messages to show the new message and any responses
    await loadMessages(appState.currentContact);
    
  } catch (error) {
    console.error('Failed to send message:', error);
    alert('Failed to send message: ' + error);
  }
}

// Change layout position
function changeLayout(position) {
  const mainContent = document.getElementById('main-content');
  
  // Remove all layout classes
  mainContent.classList.remove('contacts-left', 'contacts-right', 'contacts-top', 'contacts-bottom');
  
  // Add new layout class
  mainContent.classList.add(`contacts-${position}`);
  
  appState.layoutPosition = position;
  
  // Update active button
  document.querySelectorAll('.layout-option').forEach(btn => {
    btn.classList.remove('active');
  });
  
  document.querySelector(`[data-position="${position}"]`).classList.add('active');
  
  // Save to settings
  saveSettings();
  
  closeLayoutModal();
}

// Open/close modals and menus
function openLayoutModal() {
  document.getElementById('layout-modal').classList.remove('hidden');
}

function closeLayoutModal() {
  document.getElementById('layout-modal').classList.add('hidden');
}

function openConnectDialog() {
  document.getElementById('connect-dialog').classList.remove('hidden');
  document.getElementById('peer-address').focus();
}

function closeConnectDialog() {
  document.getElementById('connect-dialog').classList.add('hidden');
  document.getElementById('peer-address').value = '';
}

function openSystemMenu() {
  if (!appState.systemMenuOpen) {
    document.getElementById('system-menu').classList.remove('hidden');
    appState.systemMenuOpen = true;
  }
}

function closeSystemMenu() {
  if (appState.systemMenuOpen) {
    document.getElementById('system-menu').classList.add('hidden');
    appState.systemMenuOpen = false;
  }
}

// Connect to a peer
async function connectToPeer() {
  const addressInput = document.getElementById('peer-address');
  const address = addressInput.value.trim();
  
  if (!address) {
    alert('Please enter a peer address');
    return;
  }
  
  try {
    const result = await invoke('connect_peer', { address });
    console.log('Connected to peer:', result);
    
    closeConnectDialog();
    
    // Reload contacts to show the new connection
    await loadContacts();
    
  } catch (error) {
    console.error('Failed to connect to peer:', error);
    alert('Failed to connect: ' + error);
  }
}

// Execute system commands
async function executeSystemCommand(command) {
  closeSystemMenu();
  
  let message = '';
  
  switch (command) {
    case 'status':
      try {
        const status = await invoke('get_network_status');
        message = `📊 Network Status\\n\\n` +
          `Connected: ${status.is_connected ? 'Yes' : 'No'}\\n` +
          `Local Address: ${status.local_address}\\n` +
          `Peers: ${status.peer_count}\\n` +
          `Bootstrap Nodes: ${status.bootstrap_nodes}`;
      } catch (error) {
        message = 'Failed to get network status: ' + error;
      }
      break;
      
    case 'peers':
      message = '👥 Connected Peers\\n\\nNo peers currently connected.';
      break;
      
    case 'tunnels':
      message = '🚇 Tunnel Information\\n\\nNo active tunnels.';
      break;
      
    case 'addresses':
      message = '🏠 Three-word Addresses\\n\\nYour address: swift.lighthouse.mountain';
      break;
      
    case 'inbox':
      try {
        const result = await invoke('create_inbox', { inboxName: 'private' });
        message = result;
      } catch (error) {
        message = 'Failed to create inbox: ' + error;
      }
      break;
      
    case 'profile':
      openProfileModal();
      return; // Don't send message for modal actions
      
    case 'contacts':
      openContactModal();
      return; // Don't send message for modal actions
      
    default:
      message = 'Unknown command: ' + command;
  }
  
  // Send the command result as a system message
  if (appState.currentContact === 'system') {
    try {
      await invoke('send_message', {
        contactId: 'system',
        content: message
      });
      
      await loadMessages('system');
    } catch (error) {
      console.error('Failed to send system message:', error);
    }
  }
}

// Settings management
function saveSettings() {
  const settings = {
    layoutPosition: appState.layoutPosition
  };
  
  localStorage.setItem('antConnectSettings', JSON.stringify(settings));
}

function loadSettings() {
  try {
    const settings = JSON.parse(localStorage.getItem('antConnectSettings') || '{}');
    
    if (settings.layoutPosition && settings.layoutPosition !== appState.layoutPosition) {
      changeLayout(settings.layoutPosition);
    }
  } catch (error) {
    console.error('Failed to load settings:', error);
  }
}

// Utility functions
function escapeHtml(text) {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

// Periodically refresh contacts and network status
setInterval(async () => {
  if (appState.networkInitialized) {
    try {
      await loadContacts();
      
      const status = await invoke('get_network_status');
      updateNetworkStatus(status.is_connected ? 'Online' : 'Offline');
    } catch (error) {
      console.error('Failed to refresh status:', error);
    }
  }
}, 30000); // Every 30 seconds

console.log('Saorsa initialized');

// ================== Profile Management Functions ==================

function openProfileModal() {
  document.getElementById('profile-modal').classList.remove('hidden');
  appState.profileModalOpen = true;
  loadUserProfile();
}

function closeProfileModal() {
  document.getElementById('profile-modal').classList.add('hidden');
  appState.profileModalOpen = false;
}

function switchProfileTab(tabName) {
  // Update tab buttons
  document.querySelectorAll('#profile-modal .tab-btn').forEach(btn => {
    btn.classList.remove('active');
  });
  document.querySelector(`#profile-modal .tab-btn[data-tab="${tabName}"]`).classList.add('active');
  
  // Update tab content
  document.querySelectorAll('#profile-modal .tab-content').forEach(content => {
    content.classList.remove('active');
  });
  document.getElementById(`${tabName}-tab`).classList.add('active');
}

async function loadUserProfile() {
  try {
    // Load user identity if available
    const identity = await invoke('get_user_identity');
    if (identity) {
      appState.userIdentity = identity;
      document.getElementById('user-id').textContent = identity.user_id;
      document.getElementById('public-key').textContent = identity.public_key.slice(0, 32) + '...';
      document.getElementById('three-word-address').value = identity.three_word_address;
      document.getElementById('display-name').value = identity.display_name_hint.split(':')[0];
      
      // Update verification status
      const verificationBadge = document.getElementById('verification-level');
      verificationBadge.textContent = identity.verification_level;
      verificationBadge.className = `verification-badge ${identity.verification_level.toLowerCase().replace('_', '-')}`;
      
      // Update IPv6 binding status
      if (identity.ipv6_binding_proof) {
        document.getElementById('ipv6-binding-status').innerHTML = '✅ Bound';
        document.getElementById('ipv6-binding-status').className = 'status-indicator success';
      }
    }
    
    // Load user profile if available
    const profile = await invoke('get_user_profile');
    if (profile) {
      appState.userProfile = profile;
      document.getElementById('display-name').value = profile.display_name;
      document.getElementById('status-message').value = profile.status_message || '';
      
      // Load preferences
      if (profile.preferences) {
        document.getElementById('discoverable-by-name').checked = profile.preferences.discovery.discoverable_by_name;
        document.getElementById('discoverable-by-friends').checked = profile.preferences.discovery.discoverable_by_friends;
        document.getElementById('allow-contact-requests').checked = profile.preferences.discovery.allow_contact_requests;
        document.getElementById('require-mutual-friends').checked = profile.preferences.discovery.require_mutual_friends;
        
        document.getElementById('default-see-name').checked = profile.preferences.default_permissions.can_see_display_name;
        document.getElementById('default-see-avatar').checked = profile.preferences.default_permissions.can_see_avatar;
        document.getElementById('default-see-status').checked = profile.preferences.default_permissions.can_see_status;
        document.getElementById('default-see-last-seen').checked = profile.preferences.default_permissions.can_see_last_seen;
        
        document.getElementById('require-proof-of-humanity').checked = profile.preferences.privacy.require_proof_of_humanity;
        document.getElementById('enable-forward-secrecy').checked = profile.preferences.privacy.enable_forward_secrecy;
        document.getElementById('auto-rotate-keys').checked = profile.preferences.privacy.auto_rotate_keys;
      }
      
      // Load avatar if available
      if (profile.avatar_hash) {
        // TODO: Load avatar from hash
      }
    }
  } catch (error) {
    console.error('Failed to load user profile:', error);
  }
}

async function saveProfile() {
  try {
    const profileData = {
      display_name: document.getElementById('display-name').value,
      status_message: document.getElementById('status-message').value,
      preferences: {
        discovery: {
          discoverable_by_name: document.getElementById('discoverable-by-name').checked,
          discoverable_by_friends: document.getElementById('discoverable-by-friends').checked,
          allow_contact_requests: document.getElementById('allow-contact-requests').checked,
          require_mutual_friends: document.getElementById('require-mutual-friends').checked,
        },
        default_permissions: {
          can_see_display_name: document.getElementById('default-see-name').checked,
          can_see_avatar: document.getElementById('default-see-avatar').checked,
          can_see_status: document.getElementById('default-see-status').checked,
          can_see_last_seen: document.getElementById('default-see-last-seen').checked,
        },
        privacy: {
          require_proof_of_humanity: document.getElementById('require-proof-of-humanity').checked,
          enable_forward_secrecy: document.getElementById('enable-forward-secrecy').checked,
          auto_rotate_keys: document.getElementById('auto-rotate-keys').checked,
        }
      }
    };
    
    await invoke('update_user_profile', { profileData });
    showNotification('Profile saved successfully!', 'success');
    closeProfileModal();
  } catch (error) {
    console.error('Failed to save profile:', error);
    showNotification('Failed to save profile: ' + error, 'error');
  }
}

async function createNewIdentity() {
  try {
    const displayName = document.getElementById('display-name').value || 'Anonymous User';
    const threeWordAddress = generateThreeWordAddress();
    
    const identity = await invoke('create_user_identity', {
      displayName,
      threeWordAddress
    });
    
    appState.userIdentity = identity;
    showNotification('New identity created successfully!', 'success');
    loadUserProfile();
  } catch (error) {
    console.error('Failed to create identity:', error);
    showNotification('Failed to create identity: ' + error, 'error');
  }
}

async function exportIdentity() {
  try {
    const identityData = await invoke('export_user_identity');
    const blob = new Blob([identityData], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    
    const a = document.createElement('a');
    a.href = url;
    a.download = `identity-${Date.now()}.json`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    
    showNotification('Identity exported successfully!', 'success');
  } catch (error) {
    console.error('Failed to export identity:', error);
    showNotification('Failed to export identity: ' + error, 'error');
  }
}

async function importIdentity(event) {
  try {
    const file = event.target.files[0];
    if (!file) return;
    
    const text = await file.text();
    await invoke('import_user_identity', { identityData: text });
    
    showNotification('Identity imported successfully!', 'success');
    loadUserProfile();
  } catch (error) {
    console.error('Failed to import identity:', error);
    showNotification('Failed to import identity: ' + error, 'error');
  }
}

async function bindIPv6Identity() {
  try {
    await invoke('bind_ipv6_identity');
    showNotification('IPv6 identity bound successfully!', 'success');
    loadUserProfile();
  } catch (error) {
    console.error('Failed to bind IPv6 identity:', error);
    showNotification('Failed to bind IPv6 identity: ' + error, 'error');
  }
}

function handleAvatarUpload(event) {
  const file = event.target.files[0];
  if (!file) return;
  
  if (!file.type.startsWith('image/')) {
    showNotification('Please select an image file', 'error');
    return;
  }
  
  if (file.size > 5 * 1024 * 1024) { // 5MB limit
    showNotification('Image file too large (max 5MB)', 'error');
    return;
  }
  
  const reader = new FileReader();
  reader.onload = (e) => {
    const preview = document.getElementById('avatar-preview');
    preview.innerHTML = `<img src="${e.target.result}" alt="Avatar" />`;
    // TODO: Upload avatar to profile
  };
  reader.readAsDataURL(file);
}

function removeAvatar() {
  const preview = document.getElementById('avatar-preview');
  preview.innerHTML = '<div class="avatar-placeholder">📷</div>';
  // TODO: Remove avatar from profile
}

function copyToClipboard(elementId) {
  const element = document.getElementById(elementId);
  const text = element.textContent;
  
  navigator.clipboard.writeText(text).then(() => {
    showNotification('Copied to clipboard!', 'success');
  }).catch(err => {
    console.error('Failed to copy to clipboard:', err);
    showNotification('Failed to copy to clipboard', 'error');
  });
}

function generateThreeWordAddress() {
  const words = ['swift', 'bright', 'clever', 'gentle', 'brave', 'wise', 'kind', 'strong'];
  const word1 = words[Math.floor(Math.random() * words.length)];
  const word2 = words[Math.floor(Math.random() * words.length)];
  const word3 = words[Math.floor(Math.random() * words.length)];
  return `${word1}.${word2}.${word3}`;
}

// ================== Contact Management Functions ==================

function openContactModal() {
  document.getElementById('contact-modal').classList.remove('hidden');
  appState.contactModalOpen = true;
  loadContactsInModal();
  loadContactRequests();
}

// Load contacts in the contact management modal
async function loadContactsInModal() {
  try {
    const contacts = await invoke('get_contacts');
    const container = document.getElementById('contact-list');
    container.innerHTML = '';
    
    if (contacts && contacts.length > 0) {
      // Group contacts by category
      const categorized = {};
      const uncategorized = [];
      
      contacts.forEach(contact => {
        if (contact.id !== 'system') {
          if (contact.category) {
            if (!categorized[contact.category]) {
              categorized[contact.category] = [];
            }
            categorized[contact.category].push(contact);
          } else {
            uncategorized.push(contact);
          }
        }
      });
      
      // Render categorized contacts
      Object.keys(categorized).sort().forEach(category => {
        const categoryDiv = document.createElement('div');
        categoryDiv.className = 'contact-category-group';
        categoryDiv.innerHTML = `<h5 style="margin: 10px 0; color: #666;">${category}</h5>`;
        container.appendChild(categoryDiv);
        
        categorized[category].forEach(contact => {
          const item = createContactListItem(contact);
          container.appendChild(item);
        });
      });
      
      // Render uncategorized contacts
      if (uncategorized.length > 0) {
        if (Object.keys(categorized).length > 0) {
          const uncatDiv = document.createElement('div');
          uncatDiv.className = 'contact-category-group';
          uncatDiv.innerHTML = `<h5 style="margin: 10px 0; color: #666;">Other</h5>`;
          container.appendChild(uncatDiv);
        }
        
        uncategorized.forEach(contact => {
          const item = createContactListItem(contact);
          container.appendChild(item);
        });
      }
    } else {
      container.innerHTML = '<div class="empty-state"><div class="empty-state-text">No contacts yet</div></div>';
    }
  } catch (error) {
    console.error('Failed to load contacts:', error);
  }
}

// Create enhanced contact list item for management modal
function createContactListItem(contact) {
  const div = document.createElement('div');
  div.className = `contact-list-item ${contact.is_blocked ? 'blocked' : ''}`;
  div.style.cssText = 'display: flex; align-items: center; padding: 10px; border-bottom: 1px solid #eee;';
  
  const displayName = contact.nickname ? 
    `${contact.nickname} (${contact.name})` : contact.name;
  
  div.innerHTML = `
    <div class="contact-avatar" style="width: 40px; height: 40px; border-radius: 50%; background: #007AFF; color: white; display: flex; align-items: center; justify-content: center; margin-right: 15px;">
      ${contact.nickname ? contact.nickname.charAt(0) : contact.name.charAt(0)}
    </div>
    <div class="contact-info" style="flex: 1;">
      <div class="contact-name" style="font-weight: 500;">${displayName}</div>
      <div class="contact-status" style="font-size: 12px; color: #666;">${contact.three_word_address}</div>
      ${contact.is_blocked ? '<span style="color: red; font-size: 12px;">🚫 Blocked</span>' : ''}
    </div>
    <div class="contact-actions" style="display: flex; gap: 5px;">
      <button class="icon-btn" onclick='viewContactProfile(${JSON.stringify(contact).replace(/'/g, "\\'")})' title="View Profile">👤</button>
      <button class="icon-btn" onclick='openEditContactModal(${JSON.stringify(contact).replace(/'/g, "\\'")})' title="Edit">✏️</button>
      ${contact.is_blocked ? 
        `<button class="icon-btn" onclick="unblockContact('${contact.id}')" title="Unblock">✅</button>` :
        `<button class="icon-btn" onclick="blockContact('${contact.id}')" title="Block">🚫</button>`
      }
      <button class="icon-btn danger" onclick="deleteContact('${contact.id}')" title="Delete">🗑️</button>
    </div>
  `;
  
  return div;
}

function closeContactModal() {
  document.getElementById('contact-modal').classList.add('hidden');
  appState.contactModalOpen = false;
}

function switchContactTab(tabName) {
  // Update tab buttons
  document.querySelectorAll('#contact-modal .tab-btn').forEach(btn => {
    btn.classList.remove('active');
  });
  document.querySelector(`#contact-modal .tab-btn[data-tab="${tabName}"]`).classList.add('active');
  
  // Update tab content
  document.querySelectorAll('#contact-modal .tab-content').forEach(content => {
    content.classList.remove('active');
  });
  document.getElementById(`${tabName}-tab`).classList.add('active');
  
  if (tabName === 'requests') {
    loadContactRequests();
  } else if (tabName === 'search') {
    clearSearchResults();
  }
}

async function refreshContacts() {
  await loadContacts();
  if (appState.contactModalOpen) {
    await loadContactsInModal();
  }
}

async function loadContactRequests() {
  try {
    const requests = await invoke('get_contact_requests');
    renderContactRequests(requests);
  } catch (error) {
    console.error('Failed to load contact requests:', error);
  }
}

function renderContactRequests(requests) {
  const pendingContainer = document.getElementById('pending-requests');
  const sentContainer = document.getElementById('sent-requests');
  
  pendingContainer.innerHTML = '';
  sentContainer.innerHTML = '';
  
  if (requests.pending && requests.pending.length > 0) {
    requests.pending.forEach(request => {
      const requestElement = createRequestElement(request, 'pending');
      pendingContainer.appendChild(requestElement);
    });
  } else {
    pendingContainer.innerHTML = '<div class="empty-state"><div class="empty-state-text">No pending requests</div></div>';
  }
  
  if (requests.sent && requests.sent.length > 0) {
    requests.sent.forEach(request => {
      const requestElement = createRequestElement(request, 'sent');
      sentContainer.appendChild(requestElement);
    });
  } else {
    sentContainer.innerHTML = '<div class="empty-state"><div class="empty-state-text">No sent requests</div></div>';
  }
}

function createRequestElement(request, type) {
  const div = document.createElement('div');
  div.className = 'request-item';
  
  const isFromMe = type === 'sent';
  const displayName = isFromMe ? request.to_user_name : request.from_user_name;
  const userId = isFromMe ? request.to_user_id : request.from_user_id;
  
  div.innerHTML = `
    <div class="contact-avatar">👤</div>
    <div class="contact-info">
      <div class="contact-name">${displayName || 'Unknown User'}</div>
      <div class="contact-status">${userId}</div>
      ${request.message ? `<div class="request-message">${request.message}</div>` : ''}
    </div>
    <div class="contact-actions">
      ${!isFromMe ? `
        <button class="secondary-btn" onclick="acceptContactRequest('${request.request_id}')">Accept</button>
        <button class="secondary-btn" onclick="rejectContactRequest('${request.request_id}')">Reject</button>
      ` : `
        <button class="secondary-btn" onclick="cancelContactRequest('${request.request_id}')">Cancel</button>
      `}
    </div>
  `;
  
  return div;
}

async function acceptContactRequest(requestId) {
  try {
    await invoke('accept_contact_request', { requestId });
    showNotification('Contact request accepted!', 'success');
    loadContactRequests();
    loadContacts();
  } catch (error) {
    console.error('Failed to accept contact request:', error);
    showNotification('Failed to accept contact request: ' + error, 'error');
  }
}

async function rejectContactRequest(requestId) {
  try {
    await invoke('reject_contact_request', { requestId });
    showNotification('Contact request rejected', 'info');
    loadContactRequests();
  } catch (error) {
    console.error('Failed to reject contact request:', error);
    showNotification('Failed to reject contact request: ' + error, 'error');
  }
}

async function cancelContactRequest(requestId) {
  try {
    await invoke('cancel_contact_request', { requestId });
    showNotification('Contact request cancelled', 'info');
    loadContactRequests();
  } catch (error) {
    console.error('Failed to cancel contact request:', error);
    showNotification('Failed to cancel contact request: ' + error, 'error');
  }
}

async function searchContacts() {
  const query = document.getElementById('search-query').value.trim();
  if (!query) {
    showNotification('Please enter a search query', 'error');
    return;
  }
  
  try {
    const results = await invoke('search_users', { query });
    renderSearchResults(results);
  } catch (error) {
    console.error('Failed to search contacts:', error);
    showNotification('Search failed: ' + error, 'error');
  }
}

function renderSearchResults(results) {
  const container = document.getElementById('search-results');
  container.innerHTML = '';
  
  if (results && results.length > 0) {
    results.forEach(user => {
      const resultElement = createSearchResultElement(user);
      container.appendChild(resultElement);
    });
  } else {
    container.innerHTML = '<div class="empty-state"><div class="empty-state-text">No users found</div><div class="empty-state-subtext">Try a different search term</div></div>';
  }
}

function createSearchResultElement(user) {
  const div = document.createElement('div');
  div.className = 'search-result-item';
  
  div.innerHTML = `
    <div class="contact-avatar">👤</div>
    <div class="search-result-info">
      <div class="search-result-name">${user.display_name || 'Unknown User'}</div>
      <div class="search-result-id">${user.user_id}</div>
      <div class="search-result-address">${user.three_word_address}</div>
    </div>
    <div class="contact-actions">
      <button class="primary-btn" onclick="sendContactRequest('${user.user_id}', '${user.display_name || 'Unknown User'}')">
        Add Contact
      </button>
    </div>
  `;
  
  return div;
}

async function sendContactRequest(userId, displayName) {
  try {
    const message = prompt(`Send a message to ${displayName}:`);
    await invoke('send_contact_request', { 
      userId, 
      message: message || 'Hi! I would like to add you as a contact.' 
    });
    showNotification('Contact request sent!', 'success');
  } catch (error) {
    console.error('Failed to send contact request:', error);
    showNotification('Failed to send contact request: ' + error, 'error');
  }
}

function clearSearchResults() {
  document.getElementById('search-results').innerHTML = '';
  document.getElementById('search-query').value = '';
}

// ================== Contact Context Menu ==================

function showContactContextMenu(event, contact) {
  // Remove any existing context menu
  const existingMenu = document.getElementById('contact-context-menu');
  if (existingMenu) {
    existingMenu.remove();
  }
  
  // Create context menu
  const menu = document.createElement('div');
  menu.id = 'contact-context-menu';
  menu.className = 'context-menu';
  menu.style.cssText = `
    position: fixed;
    left: ${event.clientX}px;
    top: ${event.clientY}px;
    background: white;
    border: 1px solid #ccc;
    border-radius: 8px;
    box-shadow: 0 2px 10px rgba(0,0,0,0.1);
    padding: 4px 0;
    z-index: 10000;
    min-width: 180px;
  `;
  
  const menuItems = [
    { icon: '👤', text: 'View Profile', action: () => viewContactProfile(contact) },
    { icon: '✏️', text: 'Edit Contact', action: () => openEditContactModal(contact) },
    { divider: true },
    contact.is_blocked ? 
      { icon: '✅', text: 'Unblock Contact', action: () => unblockContact(contact.id) } :
      { icon: '🚫', text: 'Block Contact', action: () => blockContact(contact.id) },
    { divider: true },
    { icon: '🗑️', text: 'Delete Contact', action: () => deleteContact(contact.id), danger: true }
  ];
  
  menuItems.forEach(item => {
    if (item.divider) {
      const divider = document.createElement('div');
      divider.style.cssText = 'height: 1px; background: #eee; margin: 4px 0;';
      menu.appendChild(divider);
    } else {
      const menuItem = document.createElement('div');
      menuItem.className = 'context-menu-item';
      menuItem.style.cssText = `
        padding: 8px 16px;
        cursor: pointer;
        display: flex;
        align-items: center;
        gap: 10px;
        color: ${item.danger ? '#ff3b30' : '#333'};
      `;
      menuItem.innerHTML = `<span>${item.icon}</span><span>${item.text}</span>`;
      
      menuItem.addEventListener('mouseenter', () => {
        menuItem.style.backgroundColor = '#f0f0f0';
      });
      
      menuItem.addEventListener('mouseleave', () => {
        menuItem.style.backgroundColor = 'transparent';
      });
      
      menuItem.addEventListener('click', () => {
        menu.remove();
        item.action();
      });
      
      menu.appendChild(menuItem);
    }
  });
  
  document.body.appendChild(menu);
  
  // Close menu when clicking outside
  const closeMenu = (e) => {
    if (!menu.contains(e.target)) {
      menu.remove();
      document.removeEventListener('click', closeMenu);
    }
  };
  
  setTimeout(() => {
    document.addEventListener('click', closeMenu);
  }, 0);
}

// View contact profile
function viewContactProfile(contact) {
  openContactProfileModal(contact);
}

// Open contact profile modal
async function openContactProfileModal(contact) {
  // Get full contact details
  let fullContact = contact;
  try {
    fullContact = await invoke('get_contact_details', { contactId: contact.id });
  } catch (error) {
    console.error('Failed to get contact details:', error);
  }
  
  const modal = document.createElement('div');
  modal.className = 'modal';
  modal.id = 'contact-profile-modal';
  modal.innerHTML = `
    <div class="modal-content large-modal">
      <div class="modal-header">
        <h3>Contact Profile</h3>
        <button class="close-btn">✕</button>
      </div>
      <div class="modal-body">
        <div class="profile-header" style="display: flex; align-items: center; gap: 20px; margin-bottom: 30px;">
          <div class="large-avatar" style="width: 80px; height: 80px; border-radius: 50%; background: #007AFF; color: white; display: flex; align-items: center; justify-content: center; font-size: 32px;">
            ${fullContact.nickname ? fullContact.nickname.charAt(0) : fullContact.name.charAt(0)}
          </div>
          <div>
            <h2 style="margin: 0;">${fullContact.nickname || fullContact.name}</h2>
            ${fullContact.nickname ? `<p style="margin: 5px 0; color: #666;">${fullContact.name}</p>` : ''}
            <p style="margin: 5px 0; color: #666;">${fullContact.three_word_address}</p>
            ${fullContact.category ? `<span class="badge" style="background: #e0e0e0; padding: 4px 8px; border-radius: 12px; font-size: 12px;">${fullContact.category}</span>` : ''}
          </div>
        </div>
        
        <div class="profile-section">
          <h4>Status</h4>
          <div class="info-group">
            <div class="info-item">
              <label>Online Status:</label>
              <span>${fullContact.is_online ? '🟢 Online' : '⚪ Offline'}</span>
            </div>
            <div class="info-item">
              <label>Last Seen:</label>
              <span>${new Date(fullContact.last_seen * 1000).toLocaleString()}</span>
            </div>
            <div class="info-item">
              <label>Trust Level:</label>
              <span>${Math.round(fullContact.trust_level * 100)}%</span>
            </div>
            <div class="info-item">
              <label>Added:</label>
              <span>${new Date(fullContact.added_at * 1000).toLocaleDateString()}</span>
            </div>
          </div>
        </div>
        
        ${fullContact.notes ? `
        <div class="profile-section">
          <h4>Notes</h4>
          <p style="background: #f5f5f5; padding: 10px; border-radius: 5px;">${fullContact.notes}</p>
        </div>
        ` : ''}
        
        <div class="profile-section">
          <h4>Privacy Permissions</h4>
          <div class="permissions-grid" style="display: grid; grid-template-columns: 1fr 1fr; gap: 10px;">
            <label style="display: flex; align-items: center; gap: 8px;">
              <input type="checkbox" ${fullContact.permissions.can_see_profile ? 'checked' : ''} disabled />
              Can see my profile
            </label>
            <label style="display: flex; align-items: center; gap: 8px;">
              <input type="checkbox" ${fullContact.permissions.can_see_online_status ? 'checked' : ''} disabled />
              Can see online status
            </label>
            <label style="display: flex; align-items: center; gap: 8px;">
              <input type="checkbox" ${fullContact.permissions.can_see_last_seen ? 'checked' : ''} disabled />
              Can see last seen
            </label>
            <label style="display: flex; align-items: center; gap: 8px;">
              <input type="checkbox" ${fullContact.permissions.can_see_avatar ? 'checked' : ''} disabled />
              Can see avatar
            </label>
            <label style="display: flex; align-items: center; gap: 8px;">
              <input type="checkbox" ${fullContact.permissions.can_send_messages ? 'checked' : ''} disabled />
              Can send messages
            </label>
          </div>
        </div>
        
        <div class="profile-section">
          <h4>Actions</h4>
          <div class="action-buttons" style="display: flex; gap: 10px; flex-wrap: wrap;">
            <button class="secondary-btn" onclick="openEditContactModal(${JSON.stringify(fullContact).replace(/"/g, '&quot;')})">
              ✏️ Edit Contact
            </button>
            ${fullContact.is_blocked ? `
              <button class="secondary-btn" onclick="unblockContact('${fullContact.id}')">
                ✅ Unblock Contact
              </button>
            ` : `
              <button class="secondary-btn" onclick="blockContact('${fullContact.id}')">
                🚫 Block Contact
              </button>
            `}
            <button class="secondary-btn danger" onclick="deleteContact('${fullContact.id}')">
              🗑️ Delete Contact
            </button>
          </div>
        </div>
      </div>
    </div>
  `;
  
  // Add event handlers
  modal.querySelector('.close-btn').addEventListener('click', () => modal.remove());
  modal.addEventListener('click', (e) => {
    if (e.target === modal) modal.remove();
  });
  
  document.body.appendChild(modal);
}

// Open edit contact modal
function openEditContactModal(contact) {
  const modal = createModal('Edit Contact', `
    <div class="input-group">
      <label for="edit-nickname">Nickname:</label>
      <input type="text" id="edit-nickname" value="${contact.nickname || ''}" placeholder="Optional nickname" />
    </div>
    <div class="input-group">
      <label for="edit-notes">Notes:</label>
      <textarea id="edit-notes" placeholder="Personal notes about this contact" rows="3">${contact.notes || ''}</textarea>
    </div>
    <div class="input-group">
      <label for="edit-category">Category:</label>
      <select id="edit-category">
        <option value="">None</option>
        <option value="Friends" ${contact.category === 'Friends' ? 'selected' : ''}>Friends</option>
        <option value="Family" ${contact.category === 'Family' ? 'selected' : ''}>Family</option>
        <option value="Work" ${contact.category === 'Work' ? 'selected' : ''}>Work</option>
      </select>
    </div>
  `, async () => {
    const nickname = document.getElementById('edit-nickname').value.trim() || null;
    const notes = document.getElementById('edit-notes').value.trim() || null;
    const category = document.getElementById('edit-category').value || null;
    
    try {
      await invoke('update_contact', {
        contactId: contact.id,
        nickname,
        notes,
        category
      });
      showNotification('Contact updated successfully', 'success');
      loadContacts();
      modal.remove();
    } catch (error) {
      showNotification('Failed to update contact: ' + error, 'error');
    }
  });
}

// Block contact
async function blockContact(contactId) {
  if (confirm('Are you sure you want to block this contact? They will not be able to send you messages.')) {
    try {
      await invoke('block_user', { userId: contactId });
      showNotification('Contact blocked', 'success');
      loadContacts();
    } catch (error) {
      showNotification('Failed to block contact: ' + error, 'error');
    }
  }
}

// Unblock contact
async function unblockContact(contactId) {
  try {
    await invoke('unblock_user', { userId: contactId });
    showNotification('Contact unblocked', 'success');
    loadContacts();
  } catch (error) {
    showNotification('Failed to unblock contact: ' + error, 'error');
  }
}

// Delete contact
async function deleteContact(contactId) {
  if (confirm('Are you sure you want to delete this contact? This action cannot be undone.')) {
    try {
      await invoke('delete_contact', { contactId });
      showNotification('Contact deleted', 'success');
      loadContacts();
      
      // Clear chat if this contact was selected
      if (appState.currentContact === contactId) {
        appState.currentContact = null;
        document.getElementById('current-contact-name').textContent = 'Select a contact to start chatting';
        document.getElementById('messages-list').innerHTML = '';
      }
    } catch (error) {
      showNotification('Failed to delete contact: ' + error, 'error');
    }
  }
}

// Create modal helper
function createModal(title, content, onSave) {
  const modal = document.createElement('div');
  modal.className = 'modal';
  modal.style.cssText = `
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 10000;
  `;
  
  modal.innerHTML = `
    <div class="modal-content" style="background: white; padding: 20px; border-radius: 10px; max-width: 500px; width: 90%;">
      <div class="modal-header" style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px;">
        <h3 style="margin: 0;">${title}</h3>
        <button class="close-btn" style="background: none; border: none; font-size: 20px; cursor: pointer;">✕</button>
      </div>
      <div class="modal-body">
        ${content}
      </div>
      <div class="modal-actions" style="display: flex; gap: 10px; justify-content: flex-end; margin-top: 20px;">
        <button class="secondary-btn">Cancel</button>
        <button class="primary-btn">Save</button>
      </div>
    </div>
  `;
  
  // Event handlers
  modal.querySelector('.close-btn').addEventListener('click', () => modal.remove());
  modal.querySelector('.secondary-btn').addEventListener('click', () => modal.remove());
  modal.querySelector('.primary-btn').addEventListener('click', onSave);
  
  // Close on outside click
  modal.addEventListener('click', (e) => {
    if (e.target === modal) modal.remove();
  });
  
  document.body.appendChild(modal);
  return modal;
}

// ================== Utility Functions ==================

function showNotification(message, type = 'info') {
  // Create notification element
  const notification = document.createElement('div');
  notification.className = `notification notification-${type}`;
  notification.textContent = message;
  
  // Style the notification
  notification.style.cssText = `
    position: fixed;
    top: 20px;
    right: 20px;
    padding: 12px 20px;
    border-radius: 8px;
    color: white;
    font-weight: 500;
    z-index: 10000;
    animation: slideIn 0.3s ease;
    max-width: 300px;
    word-wrap: break-word;
  `;
  
  // Set background color based on type
  switch (type) {
    case 'success':
      notification.style.backgroundColor = '#34C759';
      break;
    case 'error':
      notification.style.backgroundColor = '#FF3B30';
      break;
    case 'warning':
      notification.style.backgroundColor = '#FF9500';
      break;
    default:
      notification.style.backgroundColor = '#007AFF';
  }
  
  // Add to document
  document.body.appendChild(notification);
  
  // Remove after 3 seconds
  setTimeout(() => {
    notification.style.animation = 'slideOut 0.3s ease';
    setTimeout(() => {
      if (notification.parentNode) {
        notification.parentNode.removeChild(notification);
      }
    }, 300);
  }, 3000);
}

// Add CSS animations for notifications
const style = document.createElement('style');
style.textContent = `
  @keyframes slideIn {
    from { transform: translateX(100%); opacity: 0; }
    to { transform: translateX(0); opacity: 1; }
  }
  
  @keyframes slideOut {
    from { transform: translateX(0); opacity: 1; }
    to { transform: translateX(100%); opacity: 0; }
  }
`;
document.head.appendChild(style);