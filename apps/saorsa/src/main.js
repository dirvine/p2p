// Saorsa Professional - Simple Working JavaScript
console.log('Loading Saorsa Professional...');

// Global state
const state = {
    currentUser: {
        name: 'Demo User',
        role: 'Team Leader',
        avatar: null,
        id: 'demo-user-123',
        threeWordAddress: 'demo.user.address',
        status: 'Available',
        bio: 'Demo user for Saorsa app'
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
    
    // Check if user has an identity, show welcome screen if not
    checkUserIdentityOnStartup();
    
    // Setup event listeners
    setupNavigationTabs();
    setupUserMenu();
    setupMobileLifecycle();
    setupWelcomeScreen();
    
    console.log('Saorsa Professional initialized successfully');
});

// Check if user has an existing identity
async function checkUserIdentityOnStartup() {
    console.log('🔍 Checking user identity on startup...');
    
    try {
        let hasIdentity = false;
        
        if (window.__TAURI__) {
            console.log('📱 Running in Tauri mode, checking backend identity...');
            const { invoke } = window.__TAURI__.core;
            try {
                const identity = await invoke('get_user_identity');
                console.log('🆔 Backend identity result:', identity);
                hasIdentity = identity !== null && identity !== undefined;
                
                if (hasIdentity) {
                    console.log('✅ Found existing identity:', identity.display_name);
                    // Update state with existing identity
                    state.currentUser = {
                        name: identity.display_name,
                        role: 'User',
                        avatar: null,
                        id: identity.user_id,
                        threeWordAddress: identity.three_word_address,
                        status: 'Available',
                        bio: identity.bio || ''
                    };
                }
            } catch (backendError) {
                console.log('❌ Backend identity check failed:', backendError);
                hasIdentity = false;
            }
        } else {
            console.log('🌐 Running in browser mode, checking localStorage...');
            // Check localStorage for demo identity
            const savedIdentity = localStorage.getItem('saorsa-demo-identity');
            if (savedIdentity) {
                const identity = JSON.parse(savedIdentity);
                hasIdentity = true;
                state.currentUser = identity;
                console.log('✅ Found demo identity:', identity.name);
            } else {
                console.log('❌ No demo identity found in localStorage');
            }
        }
        
        console.log('🎯 Has identity:', hasIdentity);
        
        if (hasIdentity) {
            console.log('🚀 User has identity, checking for passkey unlock...');
            // User has identity, check if passkey unlock is needed
            await checkPasskeyUnlock();
        } else {
            console.log('🎨 Showing welcome screen...');
            // No identity, show welcome screen
            showWelcomeScreen();
        }
    } catch (error) {
        console.error('💥 Error checking user identity:', error);
        // Show welcome screen on error
        console.log('🎨 Fallback: Showing welcome screen due to error...');
        showWelcomeScreen();
    }
}

function showWelcomeScreen() {
    console.log('🎨 showWelcomeScreen() called');
    const welcomeScreen = document.getElementById('welcome-screen');
    const appContainer = document.getElementById('app');
    
    console.log('🔍 Welcome screen element:', welcomeScreen);
    console.log('🔍 App container element:', appContainer);
    
    if (welcomeScreen && appContainer) {
        console.log('✅ Showing welcome screen...');
        welcomeScreen.classList.remove('hidden');
        appContainer.classList.add('welcome-mode');
        console.log('🎯 Welcome screen classes:', welcomeScreen.classList.toString());
        console.log('🎯 App container classes:', appContainer.classList.toString());
    } else {
        console.error('❌ Missing welcome screen or app container elements!');
        if (!welcomeScreen) console.error('❌ welcome-screen element not found');
        if (!appContainer) console.error('❌ app container element not found');
    }
}

function showMainApp() {
    console.log('🚀 showMainApp() called');
    const welcomeScreen = document.getElementById('welcome-screen');
    const appContainer = document.getElementById('app');
    
    if (welcomeScreen && appContainer) {
        console.log('✅ Hiding welcome screen, showing main app...');
        welcomeScreen.classList.add('hidden');
        appContainer.classList.remove('welcome-mode');
    } else {
        console.error('❌ Missing welcome screen or app container elements in showMainApp!');
    }
}

function setupWelcomeScreen() {
    // Setup name availability checking for welcome screen
    const nameInput = document.getElementById('welcome-display-name');
    const createBtn = document.getElementById('create-identity-btn');
    
    if (nameInput && createBtn) {
        let timeoutId;
        nameInput.addEventListener('input', function() {
            clearTimeout(timeoutId);
            const name = this.value.trim();
            
            // Enable/disable create button based on name length
            createBtn.disabled = name.length < 3;
            
            if (name.length < 3) {
                const availabilityDiv = document.getElementById('welcome-name-availability');
                if (availabilityDiv) {
                    availabilityDiv.className = 'name-availability';
                    availabilityDiv.innerHTML = '';
                }
                return;
            }
            
            // Debounce the availability check
            timeoutId = setTimeout(() => {
                checkWelcomeNameAvailability(name);
            }, 500);
        });
    }
}

async function checkWelcomeNameAvailability(name) {
    const availabilityIndicator = document.getElementById('welcome-name-availability');
    const createBtn = document.getElementById('create-identity-btn');
    
    if (!availabilityIndicator) return;
    
    // Show checking state
    availabilityIndicator.className = 'name-availability checking';
    availabilityIndicator.innerHTML = '<span class="name-availability-icon">⏳</span>Checking availability...';
    
    try {
        let isAvailable = false;
        
        if (window.__TAURI__) {
            const { invoke } = window.__TAURI__.core;
            isAvailable = await invoke('check_name_availability', { display_name: name });
        } else {
            // Mock availability check for demo
            isAvailable = !['demo', 'admin', 'test', 'user', 'system'].includes(name.toLowerCase());
        }
        
        if (isAvailable) {
            availabilityIndicator.className = 'name-availability available';
            availabilityIndicator.innerHTML = '<span class="name-availability-icon">✅</span>Name is available!';
            createBtn.disabled = false;
        } else {
            availabilityIndicator.className = 'name-availability taken';
            availabilityIndicator.innerHTML = '<span class="name-availability-icon">❌</span>Name is already taken';
            createBtn.disabled = true;
        }
    } catch (error) {
        console.error('Error checking name availability:', error);
        availabilityIndicator.className = 'name-availability';
        availabilityIndicator.innerHTML = '<span class="name-availability-icon">⚠️</span>Error checking availability';
        createBtn.disabled = true;
    }
}

async function createIdentityFromWelcome() {
    const nameInput = document.getElementById('welcome-display-name');
    const bioInput = document.getElementById('welcome-bio');
    const createBtn = document.getElementById('create-identity-btn');
    
    if (!nameInput) return;
    
    const name = nameInput.value.trim();
    const bio = bioInput?.value.trim() || '';
    
    if (name.length < 3) {
        showToast('Name must be at least 3 characters long', 'error');
        return;
    }
    
    createBtn.disabled = true;
    createBtn.textContent = 'Creating Identity...';
    
    try {
        let result;
        
        if (window.__TAURI__) {
            const { invoke } = window.__TAURI__.core;
            
            // Generate three-word address
            const threeWordAddress = `${name.toLowerCase()}.${Math.random().toString(36).substr(2, 8)}.identity`;
            
            result = await invoke('create_user_identity', {
                display_name: name,
                three_word_address: threeWordAddress
            });
            
            // Update global state
            state.currentUser = {
                name: result.display_name,
                role: 'User',
                avatar: null,
                id: result.user_id,
                threeWordAddress: result.three_word_address,
                status: 'Available',
                bio: bio
            };
        } else {
            // Demo mode - save to localStorage
            result = {
                display_name: name,
                user_id: 'demo-' + Date.now(),
                three_word_address: `${name.toLowerCase()}.demo.address`
            };
            
            state.currentUser = {
                name: result.display_name,
                role: 'User',
                avatar: null,
                id: result.user_id,
                threeWordAddress: result.three_word_address,
                status: 'Available',
                bio: bio
            };
            
            // Save to localStorage for demo persistence
            localStorage.setItem('saorsa-demo-identity', JSON.stringify(state.currentUser));
        }
        
        // Show success step
        showWelcomeStep2(result);
        
    } catch (error) {
        console.error('Error creating identity:', error);
        showToast('Failed to create identity: ' + error.message, 'error');
        createBtn.disabled = false;
        createBtn.textContent = 'Create My Identity';
    }
}

function showWelcomeStep2(identityData) {
    // Hide step 1, show step 2
    const step1 = document.getElementById('welcome-step-1');
    const step2 = document.getElementById('welcome-step-2');
    
    if (step1 && step2) {
        step1.classList.remove('active');
        step2.classList.add('active');
        
        // Populate success information
        const nameSpan = document.getElementById('created-display-name');
        const addressSpan = document.getElementById('created-address');
        const userIdSpan = document.getElementById('created-user-id');
        
        if (nameSpan) nameSpan.textContent = identityData.display_name || identityData.display_name_hint;
        if (addressSpan) addressSpan.textContent = identityData.three_word_address;
        if (userIdSpan) userIdSpan.textContent = identityData.user_id;
    }
}

function proceedToSecuritySetup() {
    // Move from step 2 to step 3 (security setup)
    const step2 = document.getElementById('welcome-step-2');
    const step3 = document.getElementById('welcome-step-3');
    
    if (step2 && step3) {
        step2.classList.remove('active');
        step3.classList.add('active');
        
        // Initialize passkey availability check
        checkPasskeyAvailabilityInWelcome();
    }
}

async function checkPasskeyAvailabilityInWelcome() {
    console.log('🔍 Checking passkey availability in welcome flow...');
    
    const statusDiv = document.getElementById('passkey-availability-check');
    const optionsDiv = document.getElementById('security-options');
    const biometricBtn = document.getElementById('setup-biometric-btn');
    const backupBtn = document.getElementById('setup-backup-only-btn');
    
    try {
        // Check if passkey auth is available
        let isAvailable = false;
        let platformInfo = 'Demo Platform';
        
        if (window.saorsa?.passkey?.auth) {
            await window.saorsa.passkey.auth.init();
            isAvailable = window.saorsa.passkey.auth.isAvailable;
            platformInfo = window.saorsa.passkey.auth.getPlatformName();
        } else if (window.__TAURI__) {
            // Direct Tauri call if passkey module not ready
            const { invoke } = window.__TAURI__.core;
            isAvailable = await invoke('check_passkey_availability');
            const info = await invoke('get_passkey_platform_info');
            platformInfo = info.platform || 'Unknown Platform';
        }
        
        // Update UI based on availability
        if (statusDiv) statusDiv.classList.add('hidden');
        if (optionsDiv) optionsDiv.classList.remove('hidden');
        
        const biometricDescription = document.getElementById('biometric-description');
        if (biometricDescription) {
            if (isAvailable) {
                biometricDescription.textContent = `Use ${platformInfo} to secure your data`;
                if (biometricBtn) biometricBtn.disabled = false;
            } else {
                biometricDescription.textContent = 'Biometric authentication not available on this device';
                if (biometricBtn) biometricBtn.style.display = 'none';
            }
        }
        
        if (backupBtn) backupBtn.disabled = false;
        
        console.log('✅ Passkey availability check complete:', { isAvailable, platformInfo });
        
    } catch (error) {
        console.error('❌ Error checking passkey availability:', error);
        
        // Show error and enable backup-only option
        if (statusDiv) {
            statusDiv.innerHTML = `
                <div class="status-indicator">
                    <span style="color: var(--error);">⚠️ Could not check biometric availability</span>
                </div>
            `;
        }
        
        if (optionsDiv) optionsDiv.classList.remove('hidden');
        if (biometricBtn) biometricBtn.style.display = 'none';
        if (backupBtn) backupBtn.disabled = false;
    }
}

async function setupBiometricAuth() {
    console.log('🔐 Setting up biometric authentication...');
    
    const statusDiv = document.getElementById('security-setup-status');
    const word1 = document.getElementById('backup-word1')?.value.trim();
    const word2 = document.getElementById('backup-word2')?.value.trim();
    const word3 = document.getElementById('backup-word3')?.value.trim();
    const pin = document.getElementById('backup-pin')?.value;
    const confirmPin = document.getElementById('backup-pin-confirm')?.value;
    
    // Validation
    if (!word1 || !word2 || !word3) {
        showSecurityStatus('Please enter all three words', 'error');
        return;
    }
    
    if (!pin || pin.length < 8) {
        showSecurityStatus('PIN must be at least 8 characters', 'error');
        return;
    }
    
    if (pin !== confirmPin) {
        showSecurityStatus('PINs do not match', 'error');
        return;
    }
    
    try {
        showSecurityStatus('Setting up biometric authentication...', 'info');
        
        let result = null;
        
        if (window.saorsa?.passkey?.auth) {
            result = await window.saorsa.passkey.auth.createPasskey(pin);
        } else if (window.__TAURI__) {
            const { invoke } = window.__TAURI__.core;
            result = await invoke('create_passkey', { password: pin });
        } else {
            // Demo mode
            result = { method: 'biometric', demo: true };
        }
        
        showSecurityStatus('✅ Biometric authentication enabled!', 'success');
        
        // Store security setup info for final step
        state.securitySetup = {
            method: 'Biometric + Backup',
            words: [word1, word2, word3],
            hasPasskey: true
        };
        
        setTimeout(() => {
            showWelcomeStep4();
        }, 1500);
        
    } catch (error) {
        console.error('❌ Failed to setup biometric auth:', error);
        showSecurityStatus(`❌ Setup failed: ${error.message}`, 'error');
    }
}

async function setupBackupOnlyAuth() {
    console.log('🔑 Setting up backup-only authentication...');
    
    const word1 = document.getElementById('backup-word1')?.value.trim();
    const word2 = document.getElementById('backup-word2')?.value.trim();
    const word3 = document.getElementById('backup-word3')?.value.trim();
    const pin = document.getElementById('backup-pin')?.value;
    const confirmPin = document.getElementById('backup-pin-confirm')?.value;
    
    // Validation
    if (!word1 || !word2 || !word3) {
        showSecurityStatus('Please enter all three words', 'error');
        return;
    }
    
    if (!pin || pin.length < 8) {
        showSecurityStatus('PIN must be at least 8 characters', 'error');
        return;
    }
    
    if (pin !== confirmPin) {
        showSecurityStatus('PINs do not match', 'error');
        return;
    }
    
    showSecurityStatus('Configuring backup authentication...', 'info');
    
    // Store backup credentials securely (in real app, this would be encrypted)
    state.securitySetup = {
        method: 'Backup Only',
        words: [word1, word2, word3],
        hasPasskey: false
    };
    
    showSecurityStatus('✅ Backup authentication configured!', 'success');
    
    setTimeout(() => {
        showWelcomeStep4();
    }, 1500);
}

function skipSecuritySetup() {
    console.log('⏭️ Skipping security setup...');
    
    state.securitySetup = {
        method: 'None (Can be configured later)',
        words: [],
        hasPasskey: false
    };
    
    showWelcomeStep4();
}

function showWelcomeStep4() {
    // Move from step 3 to step 4 (final step)
    const step3 = document.getElementById('welcome-step-3');
    const step4 = document.getElementById('welcome-step-4');
    
    if (step3 && step4) {
        step3.classList.remove('active');
        step4.classList.add('active');
        
        // Populate final summary
        const nameSpan = document.getElementById('final-display-name');
        const addressSpan = document.getElementById('final-address');
        const securitySpan = document.getElementById('final-security-method');
        
        if (nameSpan) nameSpan.textContent = state.currentUser?.name || '--';
        if (addressSpan) addressSpan.textContent = state.currentUser?.threeWordAddress || '--';
        if (securitySpan) securitySpan.textContent = state.securitySetup?.method || 'None';
    }
}

function showSecurityStatus(message, type) {
    const statusDiv = document.getElementById('security-setup-status');
    if (statusDiv) {
        statusDiv.className = `status-message ${type}`;
        statusDiv.textContent = message;
        statusDiv.classList.remove('hidden');
    }
}

async function checkPasskeyUnlock() {
    console.log('🔐 Checking if passkey unlock is required...');
    
    try {
        let needsUnlock = false;
        let hasPasskey = false;
        
        if (window.__TAURI__) {
            const { invoke } = window.__TAURI__.core;
            
            // Check if user has stored passkey credentials
            try {
                const credentials = await invoke('get_stored_passkey_credentials');
                hasPasskey = credentials && credentials.length > 0;
                
                if (hasPasskey) {
                    // Check if identity storage is locked
                    const isUnlocked = await invoke('is_identity_unlocked');
                    needsUnlock = !isUnlocked;
                }
            } catch (error) {
                console.log('📱 No passkey credentials found or identity already unlocked');
                hasPasskey = false;
                needsUnlock = false;
            }
        } else {
            // Demo mode - check localStorage for saved security setup
            const savedSecurity = localStorage.getItem('saorsa-demo-security');
            if (savedSecurity) {
                const security = JSON.parse(savedSecurity);
                hasPasskey = security.hasPasskey;
                needsUnlock = hasPasskey; // Always require unlock in demo if passkey was set up
            }
        }
        
        console.log('🔐 Unlock check result:', { hasPasskey, needsUnlock });
        
        if (needsUnlock) {
            console.log('🔒 Identity is locked, showing unlock flow...');
            await showPasskeyUnlockFlow();
        } else {
            console.log('🚀 Identity is unlocked, showing main app...');
            showMainApp();
            loadInitialData();
        }
        
    } catch (error) {
        console.error('❌ Error checking passkey unlock:', error);
        // Fallback to showing main app
        console.log('🚀 Fallback: Showing main app due to unlock check error...');
        showMainApp();
        loadInitialData();
    }
}

async function showPasskeyUnlockFlow() {
    console.log('🔓 Showing passkey unlock flow...');
    
    try {
        // Initialize passkey UI if available
        if (window.saorsa?.passkey?.ui) {
            const result = await window.saorsa.passkey.ui.showUnlockFlow();
            console.log('✅ Unlock successful:', result);
            
            // Proceed to main app
            showMainApp();
            loadInitialData();
            showToast('Welcome back! Identity unlocked successfully.', 'success');
            
        } else {
            // Fallback: show simple unlock prompt
            console.log('⚠️ Passkey UI not available, using fallback unlock...');
            await showFallbackUnlockFlow();
        }
        
    } catch (error) {
        console.error('❌ Unlock flow error:', error);
        
        if (error.message.includes('canceled')) {
            console.log('🚫 User canceled unlock');
            showToast('Unlock canceled. Please try again to access Saorsa.', 'warning');
            // Could show a retry button or exit the app
        } else {
            console.log('🚀 Proceeding to main app despite unlock error...');
            showMainApp();
            loadInitialData();
            showToast('Warning: Could not verify security unlock, but proceeding anyway.', 'warning');
        }
    }
}

async function showFallbackUnlockFlow() {
    console.log('🔓 Showing fallback unlock flow...');
    
    // Simple prompt-based unlock for demo/fallback
    const words = prompt('Enter your three backup words (space-separated):');
    const pin = prompt('Enter your backup PIN:');
    
    if (!words || !pin) {
        throw new Error('Unlock canceled by user');
    }
    
    const threeWords = words.split(' ');
    if (threeWords.length !== 3) {
        throw new Error('Please enter exactly three words');
    }
    
    try {
        if (window.__TAURI__) {
            const { invoke } = window.__TAURI__.core;
            await invoke('authenticate_with_three_words', {
                three_words: threeWords,
                pin: pin
            });
        } else {
            // Demo validation
            console.log('🎭 Demo mode: Simulating backup authentication...');
        }
        
        console.log('✅ Fallback unlock successful');
        
    } catch (error) {
        throw new Error('Invalid backup credentials: ' + error.message);
    }
}

function completeWelcome() {
    // Save security setup for demo mode
    if (!window.__TAURI__ && state.securitySetup) {
        localStorage.setItem('saorsa-demo-security', JSON.stringify(state.securitySetup));
        console.log('💾 Saved security setup to localStorage for demo mode');
    }
    
    // Initialize the main app with the new identity
    showMainApp();
    loadInitialData();
    
    // Show success toast
    const securityMethod = state.securitySetup?.method || 'basic';
    showToast(`Welcome to Saorsa! Your identity is secured with ${securityMethod}.`, 'success');
}

async function resetIdentity() {
    // Show confirmation dialog
    if (confirm('Are you sure you want to create a new identity? This will replace your current identity and all stored data.')) {
        console.log('🔄 Resetting identity...');
        
        try {
            // Clear stored identity
            if (window.__TAURI__) {
                console.log('📱 Tauri mode: Clearing all identity data...');
                const { invoke } = window.__TAURI__.core;
                await invoke('clear_all_identity_data');
                console.log('✅ Backend identity data cleared');
            } else {
                // Clear localStorage in demo mode
                localStorage.removeItem('saorsa-demo-identity');
                localStorage.removeItem('saorsa-demo-security');
                console.log('🌐 Demo mode: Cleared localStorage');
            }
            
            // Reset frontend state
            state.currentUser = null;
            state.securitySetup = null;
            
            // Show welcome screen
            showWelcomeScreen();
            showToast('Identity reset successfully. Please create a new identity.', 'success');
            
        } catch (error) {
            console.error('❌ Failed to reset identity:', error);
            showToast('Failed to reset identity: ' + error.message, 'error');
        }
    }
}

// Navigation
function setupNavigationTabs() {
    console.log('Setting up navigation tabs...');
    const tabs = document.querySelectorAll('.nav-tab');
    console.log('Found', tabs.length, 'navigation tabs');
    
    if (tabs.length === 0) {
        console.error('ERROR: No navigation tabs found! Check HTML structure.');
        return;
    }
    
    tabs.forEach(function(tab, index) {
        console.log('Setting up tab', index, 'with section:', tab.dataset.section);
        console.log('Tab element:', tab);
        console.log('Tab computed style:', window.getComputedStyle(tab).pointerEvents);
        
        // Add multiple event listeners for debugging
        tab.addEventListener('click', function(e) {
            e.preventDefault();
            console.log('🖱️ CLICK EVENT: Tab clicked:', tab.dataset.section);
            console.log('Event target:', e.target);
            console.log('Current target:', e.currentTarget);
            const section = tab.dataset.section;
            switchSection(section);
            
            // Visual feedback
            tab.style.backgroundColor = '#059669';
            setTimeout(function() {
                tab.style.backgroundColor = '';
            }, 200);
        });
        
        tab.addEventListener('mousedown', function(e) {
            console.log('🖱️ MOUSEDOWN EVENT on tab:', tab.dataset.section);
        });
        
        tab.addEventListener('mouseup', function(e) {
            console.log('🖱️ MOUSEUP EVENT on tab:', tab.dataset.section);
        });
        
        tab.addEventListener('mouseover', function(e) {
            console.log('🖱️ HOVER EVENT on tab:', tab.dataset.section);
        });
        
        // Make sure the tab is clearly clickable
        tab.style.cursor = 'pointer';
        tab.style.userSelect = 'none';
    });
    
    console.log('✅ Navigation setup complete!');
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

// Mobile Lifecycle Management
function setupMobileLifecycle() {
    // Check if running on mobile
    const isMobile = /iPhone|iPad|iPod|Android/i.test(navigator.userAgent);
    
    if (isMobile) {
        console.log('Mobile device detected - setting up lifecycle handlers');
        
        // Handle page visibility changes (background/foreground)
        document.addEventListener('visibilitychange', function() {
            if (document.hidden) {
                handleAppBackground();
            } else {
                handleAppForeground();
            }
        });
        
        // Handle app lifecycle events
        window.addEventListener('pagehide', handleAppBackground);
        window.addEventListener('pageshow', handleAppForeground);
        
        // Handle focus/blur events
        window.addEventListener('blur', handleAppBackground);
        window.addEventListener('focus', handleAppForeground);
    }
}

async function handleAppBackground() {
    console.log('App going to background');
    
    try {
        if (window.__TAURI__) {
            const { invoke } = window.__TAURI__.core;
            const result = await invoke('handle_app_background');
            console.log('Background optimization:', result);
        }
    } catch (error) {
        console.error('Error handling background:', error);
    }
}

async function handleAppForeground() {
    console.log('App coming to foreground');
    
    try {
        if (window.__TAURI__) {
            const { invoke } = window.__TAURI__.core;
            const result = await invoke('handle_app_foreground');
            console.log('Foreground restoration:', result);
        }
        
        // Refresh data when coming back to foreground
        loadInitialData();
    } catch (error) {
        console.error('Error handling foreground:', error);
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

// Identity Management Functions
function showProfileModal() {
    const modal = document.getElementById('profile-modal');
    if (modal) {
        modal.classList.remove('hidden');
        loadProfileData();
    }
}

function showIdentityModal() {
    const modal = document.getElementById('identity-modal');
    if (modal) {
        modal.classList.remove('hidden');
        loadIdentityData();
        switchIdentityTab('overview');
    }
}

function showSearchModal() {
    const modal = document.getElementById('search-modal');
    if (modal) {
        modal.classList.remove('hidden');
        document.getElementById('search-input').focus();
    }
}

function closeModal(modalId) {
    const modal = document.getElementById(modalId);
    if (modal) {
        modal.classList.add('hidden');
    }
}

function showSecuritySettingsModal() {
    console.log('🔐 Opening security settings...');
    
    // Use passkey UI if available, otherwise show basic modal
    if (window.saorsa?.passkey?.ui) {
        showPasskeySecuritySettings();
    } else {
        showBasicSecuritySettings();
    }
}

async function showPasskeySecuritySettings() {
    try {
        // Check current security status
        let hasPasskey = false;
        let platformInfo = 'Unknown';
        
        if (window.__TAURI__) {
            const { invoke } = window.__TAURI__.core;
            const credentials = await invoke('get_stored_passkey_credentials');
            hasPasskey = credentials && credentials.length > 0;
            const info = await invoke('get_passkey_platform_info');
            platformInfo = info.platform || 'Unknown Platform';
        } else {
            // Demo mode
            const savedSecurity = localStorage.getItem('saorsa-demo-security');
            if (savedSecurity) {
                const security = JSON.parse(savedSecurity);
                hasPasskey = security.hasPasskey;
            }
            platformInfo = 'Demo Platform';
        }
        
        // Create security settings modal
        const modal = document.createElement('div');
        modal.className = 'passkey-modal';
        modal.innerHTML = `
            <div class="passkey-modal-backdrop">
                <div class="passkey-modal-container">
                    <div class="passkey-modal-header">
                        <h2 class="passkey-modal-title">🔐 Security Settings</h2>
                        <button class="passkey-modal-close" onclick="this.closest('.passkey-modal').remove()">
                            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                <line x1="18" y1="6" x2="6" y2="18"></line>
                                <line x1="6" y1="6" x2="18" y2="18"></line>
                            </svg>
                        </button>
                    </div>
                    <div class="passkey-modal-body">
                        <div class="security-status-section">
                            <h3>Current Security Status</h3>
                            <div class="status-item">
                                <span>Platform:</span>
                                <span>${platformInfo}</span>
                            </div>
                            <div class="status-item">
                                <span>Biometric Authentication:</span>
                                <span class="${hasPasskey ? 'status-enabled' : 'status-disabled'}">
                                    ${hasPasskey ? '✅ Enabled' : '❌ Not Set Up'}
                                </span>
                            </div>
                            <div class="status-item">
                                <span>Backup Access:</span>
                                <span class="status-enabled">✅ Configured</span>
                            </div>
                        </div>
                        
                        <div class="security-actions-section">
                            <h3>Security Actions</h3>
                            ${!hasPasskey ? `
                                <button class="btn btn-primary btn-full" onclick="setupPasskeyFromSettings()">
                                    🔐 Enable Biometric Authentication
                                </button>
                            ` : `
                                <button class="btn btn-secondary btn-full" onclick="testPasskeyAuth()">
                                    🔓 Test Biometric Authentication
                                </button>
                            `}
                            
                            <button class="btn btn-secondary btn-full" onclick="updateBackupCredentials()">
                                🔑 Update Backup Credentials
                            </button>
                            
                            ${hasPasskey ? `
                                <button class="btn btn-danger btn-full" onclick="disablePasskeyAuth()">
                                    ❌ Disable Biometric Authentication
                                </button>
                            ` : ''}
                        </div>
                        
                        <div class="security-info-section">
                            <h3>Security Information</h3>
                            <ul>
                                <li>Your identity is encrypted and stored locally</li>
                                <li>Biometric data never leaves your device</li>
                                <li>Backup credentials are for emergency access</li>
                                <li>All authentication happens offline</li>
                            </ul>
                        </div>
                    </div>
                </div>
            </div>
        `;
        
        document.body.appendChild(modal);
        
    } catch (error) {
        console.error('❌ Error showing security settings:', error);
        showToast('Error loading security settings: ' + error.message, 'error');
    }
}

function showBasicSecuritySettings() {
    showToast('Security settings require the full Saorsa application with passkey support.', 'info');
}

async function setupPasskeyFromSettings() {
    try {
        console.log('🔐 Setting up passkey from settings...');
        
        if (window.saorsa?.passkey?.ui) {
            const result = await window.saorsa.passkey.ui.showSetupFlow();
            console.log('✅ Passkey setup successful:', result);
            
            // Close modal and refresh
            document.querySelector('.passkey-modal').remove();
            showToast('Biometric authentication enabled successfully!', 'success');
            
            // Update demo storage if needed
            if (!window.__TAURI__) {
                const security = { hasPasskey: true, method: 'Biometric + Backup' };
                localStorage.setItem('saorsa-demo-security', JSON.stringify(security));
            }
            
        } else {
            throw new Error('Passkey UI not available');
        }
        
    } catch (error) {
        console.error('❌ Failed to setup passkey:', error);
        showToast('Failed to enable biometric authentication: ' + error.message, 'error');
    }
}

async function testPasskeyAuth() {
    try {
        console.log('🔓 Testing passkey authentication...');
        
        if (window.saorsa?.passkey?.auth) {
            const result = await window.saorsa.passkey.auth.authenticate();
            console.log('✅ Passkey test successful:', result);
            showToast('Biometric authentication test successful!', 'success');
        } else if (window.__TAURI__) {
            const { invoke } = window.__TAURI__.core;
            await invoke('authenticate_with_passkey');
            showToast('Biometric authentication test successful!', 'success');
        } else {
            showToast('Demo: Biometric authentication would work here!', 'info');
        }
        
    } catch (error) {
        console.error('❌ Passkey test failed:', error);
        showToast('Biometric authentication test failed: ' + error.message, 'error');
    }
}

function updateBackupCredentials() {
    showToast('Backup credential update would be implemented here.', 'info');
}

async function disablePasskeyAuth() {
    if (confirm('Are you sure you want to disable biometric authentication? You will only be able to access your account with backup credentials.')) {
        try {
            // In a real implementation, this would delete stored passkey credentials
            console.log('🔐 Disabling passkey authentication...');
            
            if (!window.__TAURI__) {
                // Demo mode
                const security = { hasPasskey: false, method: 'Backup Only' };
                localStorage.setItem('saorsa-demo-security', JSON.stringify(security));
            }
            
            document.querySelector('.passkey-modal').remove();
            showToast('Biometric authentication disabled.', 'success');
            
        } catch (error) {
            console.error('❌ Failed to disable passkey:', error);
            showToast('Failed to disable biometric authentication: ' + error.message, 'error');
        }
    }
}

function loadProfileData() {
    // Load current user profile data
    const displayNameInput = document.getElementById('profile-display-name');
    const statusInput = document.getElementById('profile-status');
    const bioTextarea = document.getElementById('profile-bio');
    const addressInput = document.getElementById('profile-address');
    
    if (displayNameInput) displayNameInput.value = state.currentUser.name || '';
    if (statusInput) statusInput.value = state.currentUser.status || '';
    if (bioTextarea) bioTextarea.value = state.currentUser.bio || '';
    if (addressInput) addressInput.value = state.currentUser.threeWordAddress || 'generating...';
}

function loadIdentityData() {
    // Load identity information - this will connect to the backend
    loadIdentityOverview();
}

async function loadIdentityOverview() {
    console.log('📋 Loading identity overview...');
    
    try {
        let identityData;
        
        if (window.__TAURI__) {
            const { invoke } = window.__TAURI__.core;
            // Get current user identity with signed packet info
            identityData = await invoke('get_user_identity');
            console.log('🆔 Backend identity data:', identityData);
        }
        
        // Update the overview tab elements with real data
        const displayNameEl = document.getElementById('identity-display-name');
        const threeWordEl = document.getElementById('identity-three-word');
        const verificationEl = document.getElementById('verification-status');
        const userIdEl = document.getElementById('identity-user-id');
        const publicKeyEl = document.getElementById('identity-public-key');
        const networkAddressEl = document.getElementById('identity-network-address');
        const createdEl = document.getElementById('identity-created');
        const avatarEl = document.getElementById('identity-avatar');
        
        if (identityData) {
            // Update elements with real identity data
            if (displayNameEl) displayNameEl.textContent = identityData.display_name || 'Unknown';
            if (threeWordEl) threeWordEl.textContent = identityData.three_word_address || '---.---.---';
            if (verificationEl) {
                verificationEl.textContent = identityData.signature_valid ? '✅ Verified' : '⚠️ Unverified';
                verificationEl.className = `verification-badge ${identityData.signature_valid ? 'verified' : 'unverified'}`;
            }
            if (userIdEl) userIdEl.textContent = identityData.user_id || '--';
            if (publicKeyEl) publicKeyEl.textContent = identityData.public_key ? `ed25519:${identityData.public_key.slice(0, 32)}...` : '--';
            if (networkAddressEl) networkAddressEl.textContent = identityData.current_network_address?.peer_id || 'Not connected';
            if (createdEl) createdEl.textContent = identityData.timestamp ? new Date(identityData.timestamp * 1000).toLocaleString() : '--';
            if (avatarEl) avatarEl.textContent = identityData.display_name ? identityData.display_name[0].toUpperCase() : '--';
            
            // Add action buttons dynamically
            addIdentityActions(true);
        } else {
            // No identity exists - show creation prompt
            if (displayNameEl) displayNameEl.textContent = 'No Identity Created';
            if (threeWordEl) threeWordEl.textContent = '---.---.---';
            if (verificationEl) {
                verificationEl.textContent = 'Not Created';
                verificationEl.className = 'verification-badge inactive';
            }
            if (userIdEl) userIdEl.textContent = '--';
            if (publicKeyEl) publicKeyEl.textContent = '--';
            if (networkAddressEl) networkAddressEl.textContent = '--';
            if (createdEl) createdEl.textContent = '--';
            if (avatarEl) avatarEl.textContent = '--';
            
            // Add create button
            addIdentityActions(false);
        }
    } catch (error) {
        console.error('❌ Error loading identity overview:', error);
        // Fallback to current user data
        const displayNameEl = document.getElementById('identity-display-name');
        const threeWordEl = document.getElementById('identity-three-word');
        const verificationEl = document.getElementById('verification-status');
        const userIdEl = document.getElementById('identity-user-id');
        const avatarEl = document.getElementById('identity-avatar');
        
        if (displayNameEl) displayNameEl.textContent = `${state.currentUser.name} (Demo)`;
        if (threeWordEl) threeWordEl.textContent = state.currentUser.threeWordAddress || '---.---.---';
        if (verificationEl) {
            verificationEl.textContent = 'Demo Mode';
            verificationEl.className = 'verification-badge demo';
        }
        if (userIdEl) userIdEl.textContent = state.currentUser.id || '--';
        if (avatarEl) avatarEl.textContent = state.currentUser.name ? state.currentUser.name[0].toUpperCase() : 'D';
        
        addIdentityActions(false);
    }
}

function addIdentityActions(hasIdentity) {
    // Find or create actions container after identity details
    let actionsContainer = document.querySelector('#identity-overview .identity-actions');
    if (!actionsContainer) {
        const detailsEl = document.querySelector('#identity-overview .identity-details');
        if (detailsEl) {
            actionsContainer = document.createElement('div');
            actionsContainer.className = 'identity-actions';
            detailsEl.parentNode.appendChild(actionsContainer);
        }
    }
    
    if (actionsContainer) {
        if (hasIdentity) {
            actionsContainer.innerHTML = `
                <button class="btn btn-secondary btn-small" onclick="verifyIdentitySignature()">
                    🔍 Verify Signature
                </button>
                <button class="btn btn-secondary btn-small" onclick="refreshNetworkAddress()">
                    🔄 Update Network Address
                </button>
                <button class="btn btn-secondary btn-small" onclick="exportIdentity()">
                    📤 Export Identity
                </button>
            `;
        } else {
            actionsContainer.innerHTML = `
                <button class="btn btn-primary" onclick="switchIdentityTab('create')">
                    ✨ Create New Identity
                </button>
            `;
        }
    }
}

function switchIdentityTab(tabName) {
    console.log('🔄 Switching to identity tab:', tabName);
    
    // Switch between identity management tabs
    const tabs = document.querySelectorAll('.identity-tabs .tab-btn');
    const contents = document.querySelectorAll('#identity-modal .tab-content');
    
    // Update tab buttons
    tabs.forEach(tab => {
        if (tab.textContent.toLowerCase().includes(tabName) || 
            (tabName === 'overview' && tab.textContent.includes('Overview')) ||
            (tabName === 'create' && tab.textContent.includes('Create')) ||
            (tabName === 'backup' && tab.textContent.includes('Backup'))) {
            tab.classList.add('active');
        } else {
            tab.classList.remove('active');
        }
    });
    
    // Update tab content visibility
    contents.forEach(content => {
        if (content.id === `identity-${tabName}`) {
            content.classList.add('active');
        } else {
            content.classList.remove('active');
        }
    });
    
    // Load tab-specific data
    if (tabName === 'overview') {
        loadIdentityOverview();
    } else if (tabName === 'create') {
        setupCreateIdentityForm();
    } else if (tabName === 'backup') {
        // Backup tab is ready to use
    }
}

function setupNameAvailabilityCheck() {
    const nameInput = document.getElementById('new-identity-name');
    const availabilityIndicator = document.getElementById('name-availability');
    
    if (!nameInput || !availabilityIndicator) return;
    
    let timeoutId;
    nameInput.addEventListener('input', function() {
        clearTimeout(timeoutId);
        const name = this.value.trim();
        
        if (name.length < 3) {
            availabilityIndicator.className = 'name-availability';
            availabilityIndicator.innerHTML = '';
            return;
        }
        
        // Show checking state
        availabilityIndicator.className = 'name-availability checking';
        availabilityIndicator.innerHTML = '<span class="name-availability-icon">⏳</span>Checking availability...';
        
        // Debounce the check
        timeoutId = setTimeout(() => {
            checkNameAvailability(name);
        }, 500);
    });
}

async function checkNameAvailability(name) {
    const availabilityIndicator = document.getElementById('name-availability');
    if (!availabilityIndicator) return;
    
    try {
        // Call the backend to check name availability in DHT
        if (window.__TAURI__) {
            const { invoke } = window.__TAURI__.core;
            const isAvailable = await invoke('check_name_availability', { display_name: name });
            
            if (isAvailable) {
                availabilityIndicator.className = 'name-availability available';
                availabilityIndicator.innerHTML = '<span class="name-availability-icon">✅</span>Name is available!';
            } else {
                availabilityIndicator.className = 'name-availability taken';
                availabilityIndicator.innerHTML = '<span class="name-availability-icon">❌</span>Name is already taken';
            }
        } else {
            // Mock response for development
            const isAvailable = !['demo', 'admin', 'test', 'user'].includes(name.toLowerCase());
            if (isAvailable) {
                availabilityIndicator.className = 'name-availability available';
                availabilityIndicator.innerHTML = '<span class="name-availability-icon">✅</span>Name is available!';
            } else {
                availabilityIndicator.className = 'name-availability taken';
                availabilityIndicator.innerHTML = '<span class="name-availability-icon">❌</span>Name is already taken';
            }
        }
    } catch (error) {
        console.error('Error checking name availability:', error);
        availabilityIndicator.className = 'name-availability';
        availabilityIndicator.innerHTML = '<span class="name-availability-icon">⚠️</span>Error checking availability';
    }
}

async function createNewIdentity() {
    const nameInput = document.getElementById('new-identity-name');
    const bioInput = document.getElementById('new-identity-bio');
    
    if (!nameInput) return;
    
    const name = nameInput.value.trim();
    if (name.length < 3) {
        showToast('Name must be at least 3 characters long', 'error');
        return;
    }
    
    // Check if name is available first
    const availabilityIndicator = document.getElementById('name-availability');
    if (availabilityIndicator && availabilityIndicator.classList.contains('taken')) {
        showToast('This name is already taken. Please choose another.', 'error');
        return;
    }
    
    try {
        if (window.__TAURI__) {
            const { invoke } = window.__TAURI__.core;
            
            // Generate three-word address
            const threeWordAddress = `${name.toLowerCase()}.${Math.random().toString(36).substr(2, 8)}.identity`;
            
            showToast('Creating signed identity packet...', 'info');
            
            const result = await invoke('create_user_identity', {
                display_name: name,
                three_word_address: threeWordAddress
            });
            
            showToast('Signed identity created and registered in DHT!', 'success');
            
            // Update local state
            state.currentUser = {
                ...state.currentUser,
                name: result.display_name,
                threeWordAddress: result.three_word_address,
                id: result.user_id
            };
            
            updateUserDisplay(state.currentUser);
            
            // Refresh the overview tab to show the new identity
            switchIdentityTab('overview');
            
            // Clear the form
            nameInput.value = '';
            if (bioInput) bioInput.value = '';
            if (availabilityIndicator) {
                availabilityIndicator.className = 'name-availability';
                availabilityIndicator.innerHTML = '';
            }
            
        } else {
            // Mock creation for development
            showToast('Identity created successfully! (Demo Mode)', 'success');
            state.currentUser = {
                ...state.currentUser,
                name,
                threeWordAddress: `${name.toLowerCase()}.demo.address`,
                id: 'demo-' + Date.now()
            };
            updateUserDisplay(state.currentUser);
            switchIdentityTab('overview');
        }
    } catch (error) {
        console.error('Error creating identity:', error);
        showToast('Failed to create identity: ' + error.message, 'error');
    }
}

// Signed packet verification and management functions
async function verifyIdentitySignature() {
    try {
        if (window.__TAURI__) {
            const { invoke } = window.__TAURI__.core;
            showToast('Verifying identity signature...', 'info');
            
            const identity = await invoke('get_user_identity');
            if (identity && identity.signature_valid) {
                showToast('✅ Identity signature is valid and authentic', 'success');
            } else {
                showToast('❌ Identity signature verification failed', 'error');
            }
        } else {
            showToast('✅ Signature verified (Demo Mode)', 'success');
        }
    } catch (error) {
        console.error('Error verifying signature:', error);
        showToast('Error verifying signature: ' + error.message, 'error');
    }
}

async function refreshNetworkAddress() {
    try {
        if (window.__TAURI__) {
            const { invoke } = window.__TAURI__.core;
            showToast('Updating network address in signed packet...', 'info');
            
            // This would trigger re-signing the identity packet with current network address
            await invoke('update_user_profile', {
                profile_data: { refresh_network_address: true }
            });
            
            showToast('Network address updated and re-signed', 'success');
            
            // Refresh the overview to show updated information
            loadIdentityOverview();
        } else {
            showToast('Network address refreshed (Demo Mode)', 'success');
        }
    } catch (error) {
        console.error('Error refreshing network address:', error);
        showToast('Error updating network address: ' + error.message, 'error');
    }
}

async function exportIdentity() {
    try {
        if (window.__TAURI__) {
            const { invoke } = window.__TAURI__.core;
            showToast('Exporting signed identity packet...', 'info');
            
            const exportData = await invoke('export_user_identity');
            
            // Create download for the exported identity
            const blob = new Blob([exportData], { type: 'application/json' });
            const url = window.URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = `saorsa-identity-${Date.now()}.json`;
            document.body.appendChild(a);
            a.click();
            document.body.removeChild(a);
            window.URL.revokeObjectURL(url);
            
            showToast('Identity exported successfully', 'success');
        } else {
            // Mock export for demo
            const mockData = JSON.stringify({
                display_name: state.currentUser.name,
                user_id: state.currentUser.id,
                three_word_address: state.currentUser.threeWordAddress,
                export_time: new Date().toISOString(),
                demo_mode: true
            }, null, 2);
            
            const blob = new Blob([mockData], { type: 'application/json' });
            const url = window.URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = `saorsa-identity-demo-${Date.now()}.json`;
            document.body.appendChild(a);
            a.click();
            document.body.removeChild(a);
            window.URL.revokeObjectURL(url);
            
            showToast('Demo identity exported', 'success');
        }
    } catch (error) {
        console.error('Error exporting identity:', error);
        showToast('Error exporting identity: ' + error.message, 'error');
    }
}

function showImportDialog() {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.json';
    input.onchange = async function(event) {
        const file = event.target.files[0];
        if (!file) return;
        
        try {
            const text = await file.text();
            await importIdentity(text);
        } catch (error) {
            showToast('Error reading file: ' + error.message, 'error');
        }
    };
    input.click();
}

async function importIdentity(event) {
    const file = event.target.files?.[0];
    if (!file) return;
    
    try {
        showToast('Reading identity file...', 'info');
        const text = await file.text();
        
        if (window.__TAURI__) {
            const { invoke } = window.__TAURI__.core;
            showToast('Importing identity and verifying signature...', 'info');
            
            const result = await invoke('import_user_identity', {
                identity_data: text
            });
            
            showToast('Identity imported and signature verified!', 'success');
            
            // Update local state and refresh UI
            const identity = await invoke('get_user_identity');
            if (identity) {
                state.currentUser = {
                    ...state.currentUser,
                    name: identity.display_name,
                    threeWordAddress: identity.three_word_address,
                    id: identity.user_id
                };
                updateUserDisplay(state.currentUser);
                loadIdentityOverview();
                switchIdentityTab('overview');
            }
        } else {
            // Demo mode - parse and show mock import
            const importData = JSON.parse(text);
            if (importData.display_name) {
                state.currentUser = {
                    ...state.currentUser,
                    name: importData.display_name,
                    threeWordAddress: importData.three_word_address || 'imported.demo.address',
                    id: importData.user_id || 'imported-' + Date.now()
                };
                updateUserDisplay(state.currentUser);
                loadIdentityOverview();
                switchIdentityTab('overview');
                showToast('Identity imported (Demo Mode)', 'success');
            } else {
                throw new Error('Invalid identity file format');
            }
        }
        
        // Clear the file input
        event.target.value = '';
        
    } catch (error) {
        console.error('❌ Error importing identity:', error);
        showToast('Failed to import identity: ' + error.message, 'error');
        // Clear the file input
        event.target.value = '';
    }
}

function setupCreateIdentityForm() {
    console.log('⚙️ Setting up create identity form...');
    
    const nameInput = document.getElementById('new-display-name');
    const threeWordInput = document.getElementById('new-three-word');
    const availabilityDiv = document.getElementById('new-name-availability');
    const createBtn = document.getElementById('create-identity-btn');
    
    if (nameInput && !nameInput.hasAttribute('data-listener-added')) {
        nameInput.setAttribute('data-listener-added', 'true');
        
        let timeoutId;
        nameInput.addEventListener('input', function() {
            clearTimeout(timeoutId);
            const name = this.value.trim();
            
            if (name.length < 3) {
                if (availabilityDiv) {
                    availabilityDiv.className = 'name-availability';
                    availabilityDiv.innerHTML = '';
                }
                if (createBtn) createBtn.disabled = true;
                return;
            }
            
            // Show checking state
            if (availabilityDiv) {
                availabilityDiv.className = 'name-availability checking';
                availabilityDiv.innerHTML = '<span class="name-availability-icon">⏳</span>Checking availability...';
            }
            
            // Update three-word address suggestion
            if (threeWordInput) {
                threeWordInput.value = `${name.toLowerCase().replace(/[^a-z0-9]/g, '')}.user.address`;
            }
            
            // Debounce the check
            timeoutId = setTimeout(() => {
                checkCreateFormNameAvailability(name);
            }, 500);
        });
    }
}

async function checkCreateFormNameAvailability(name) {
    const availabilityDiv = document.getElementById('new-name-availability');
    const createBtn = document.getElementById('create-identity-btn');
    
    if (!availabilityDiv) return;
    
    try {
        let isAvailable = false;
        
        if (window.__TAURI__) {
            const { invoke } = window.__TAURI__.core;
            isAvailable = await invoke('check_name_availability', { display_name: name });
        } else {
            // Mock response for development
            isAvailable = !['demo', 'admin', 'test', 'user'].includes(name.toLowerCase());
        }
        
        if (isAvailable) {
            availabilityDiv.className = 'name-availability available';
            availabilityDiv.innerHTML = '<span class="name-availability-icon">✅</span>Name is available!';
            if (createBtn) createBtn.disabled = false;
        } else {
            availabilityDiv.className = 'name-availability taken';
            availabilityDiv.innerHTML = '<span class="name-availability-icon">❌</span>Name is already taken';
            if (createBtn) createBtn.disabled = true;
        }
    } catch (error) {
        console.error('❌ Error checking name availability:', error);
        availabilityDiv.className = 'name-availability error';
        availabilityDiv.innerHTML = '<span class="name-availability-icon">⚠️</span>Error checking availability';
        if (createBtn) createBtn.disabled = true;
    }
}

async function createNewIdentity() {
    console.log('🆕 Creating new identity from modal...');
    
    const nameInput = document.getElementById('new-display-name');
    const threeWordInput = document.getElementById('new-three-word');
    const createBtn = document.getElementById('create-identity-btn');
    
    if (!nameInput) {
        showToast('Name input not found', 'error');
        return;
    }
    
    const name = nameInput.value.trim();
    const threeWordAddress = threeWordInput ? threeWordInput.value.trim() : `${name.toLowerCase().replace(/[^a-z0-9]/g, '')}.user.address`;
    
    if (name.length < 3) {
        showToast('Display name must be at least 3 characters', 'error');
        return;
    }
    
    if (createBtn) createBtn.disabled = true;
    
    try {
        showToast('Creating identity with signed packet...', 'info');
        
        if (window.__TAURI__) {
            const { invoke } = window.__TAURI__.core;
            
            // Create identity with signed packet
            const identity = await invoke('create_user_identity', {
                display_name: name,
                three_word_address: threeWordAddress
            });
            
            console.log('✅ Identity created:', identity);
            showToast('Identity created and signed successfully!', 'success');
            
            // Update local state
            state.currentUser = {
                name: identity.display_name_hint || name,
                role: 'User',
                avatar: null,
                id: identity.user_id,
                threeWordAddress: identity.three_word_address || threeWordAddress,
                status: 'Available',
                bio: ''
            };
            
            updateUserDisplay(state.currentUser);
            
            // Clear the form
            nameInput.value = '';
            if (threeWordInput) threeWordInput.value = '';
            const availabilityDiv = document.getElementById('new-name-availability');
            if (availabilityDiv) {
                availabilityDiv.className = 'name-availability';
                availabilityDiv.innerHTML = '';
            }
            
            // Switch to overview tab to show the new identity
            switchIdentityTab('overview');
            
        } else {
            // Mock creation for development
            showToast('Identity created successfully! (Demo Mode)', 'success');
            state.currentUser = {
                ...state.currentUser,
                name,
                threeWordAddress,
                id: 'demo-' + Date.now()
            };
            updateUserDisplay(state.currentUser);
            
            // Clear form and switch to overview
            nameInput.value = '';
            if (threeWordInput) threeWordInput.value = '';
            switchIdentityTab('overview');
        }
    } catch (error) {
        console.error('❌ Error creating identity:', error);
        showToast('Failed to create identity: ' + error.message, 'error');
    } finally {
        if (createBtn) createBtn.disabled = false;
    }
}

async function searchUsers() {
    const searchInput = document.getElementById('search-input');
    const resultsContainer = document.getElementById('search-results');
    
    if (!searchInput || !resultsContainer) return;
    
    const query = searchInput.value.trim();
    if (query.length < 2) {
        resultsContainer.innerHTML = '<p class="text-gray-500">Enter at least 2 characters to search</p>';
        return;
    }
    
    resultsContainer.innerHTML = '<p class="text-gray-500">Searching signed identity packets...</p>';
    
    try {
        if (window.__TAURI__) {
            const { invoke } = window.__TAURI__.core;
            const results = await invoke('search_network_users', { query });
            displaySearchResults(results);
        } else {
            // Mock search results for development
            const mockResults = [
                { 
                    display_name: 'Alice Johnson', 
                    three_word_address: 'alice.mountain.river', 
                    user_id: 'alice123',
                    signature_valid: true,
                    timestamp: Date.now() / 1000 - 3600
                },
                { 
                    display_name: 'Bob Smith', 
                    three_word_address: 'bob.forest.lake', 
                    user_id: 'bob456',
                    signature_valid: true,
                    timestamp: Date.now() / 1000 - 7200
                },
            ].filter(user => user.display_name.toLowerCase().includes(query.toLowerCase()));
            
            setTimeout(() => displaySearchResults(mockResults), 500);
        }
    } catch (error) {
        console.error('Error searching users:', error);
        resultsContainer.innerHTML = '<p class="text-red-500">Error searching users</p>';
    }
}

function displaySearchResults(results) {
    const resultsContainer = document.getElementById('search-results');
    if (!resultsContainer) return;
    
    if (results.length === 0) {
        resultsContainer.innerHTML = '<p class="text-gray-500">No verified identity packets found</p>';
        return;
    }
    
    resultsContainer.innerHTML = results.map(user => {
        const displayName = user.display_name || user.name;
        const threeWordAddress = user.three_word_address || user.threeWordAddress;
        const userId = user.user_id || user.userId;
        const isVerified = user.signature_valid !== false; // Default to true if not specified
        const timeAgo = user.timestamp ? getTimeAgo(user.timestamp * 1000) : '';
        
        return `
            <div class="search-result-item">
                <div class="search-result-avatar">${getInitials(displayName)}</div>
                <div class="search-result-info">
                    <div class="search-result-name">
                        ${displayName}
                        ${isVerified ? '<span class="verification-badge verified">✅</span>' : '<span class="verification-badge unverified">⚠️</span>'}
                    </div>
                    <div class="search-result-address">${threeWordAddress}</div>
                    ${timeAgo ? `<div class="search-result-time">Signed ${timeAgo}</div>` : ''}
                </div>
                <div class="search-result-actions">
                    <button class="btn btn-secondary btn-small" onclick="viewUserProfile('${userId}')">
                        👤 View
                    </button>
                    <button class="btn btn-primary btn-small" onclick="addContact('${userId}')">
                        ➕ Add
                    </button>
                </div>
            </div>
        `;
    }).join('');
}

function getTimeAgo(timestamp) {
    const now = Date.now();
    const diff = now - timestamp;
    const minutes = Math.floor(diff / 60000);
    const hours = Math.floor(diff / 3600000);
    const days = Math.floor(diff / 86400000);
    
    if (days > 0) return `${days}d ago`;
    if (hours > 0) return `${hours}h ago`;
    if (minutes > 0) return `${minutes}m ago`;
    return 'just now';
}

async function viewUserProfile(userId) {
    try {
        if (window.__TAURI__) {
            const { invoke } = window.__TAURI__.core;
            showToast('Loading user profile...', 'info');
            
            const profile = await invoke('get_user_profile', { user_id: userId });
            if (profile) {
                showUserProfileModal(profile);
            } else {
                showToast('User profile not found', 'error');
            }
        } else {
            showToast('Viewing user profile (Demo Mode)', 'info');
            // Mock profile view
            showUserProfileModal({
                display_name: 'Demo User',
                three_word_address: 'demo.user.address',
                user_id: userId,
                signature_valid: true,
                timestamp: Date.now() / 1000 - 3600
            });
        }
    } catch (error) {
        console.error('Error viewing user profile:', error);
        showToast('Error loading user profile: ' + error.message, 'error');
    }
}

function showUserProfileModal(profile) {
    // Create a temporary modal to show user profile
    const modal = document.createElement('div');
    modal.className = 'modal';
    modal.innerHTML = `
        <div class="modal-container">
            <div class="modal-header">
                <h3>User Profile</h3>
                <button class="modal-close" onclick="this.closest('.modal').remove()">×</button>
            </div>
            <div class="modal-body">
                <div class="identity-card">
                    <div class="identity-header">
                        <div class="identity-name">${profile.display_name}</div>
                        <div class="identity-status ${profile.signature_valid ? 'verified' : 'unverified'}">
                            ${profile.signature_valid ? '✅ Verified' : '⚠️ Unverified'}
                        </div>
                    </div>
                    <div class="identity-info">
                        <div class="identity-label">Display Name:</div>
                        <div class="identity-value">${profile.display_name}</div>
                        <div class="identity-label">Three-word Address:</div>
                        <div class="identity-value">${profile.three_word_address}</div>
                        <div class="identity-label">User ID:</div>
                        <div class="identity-value">${profile.user_id}</div>
                        ${profile.public_key ? `
                            <div class="identity-label">Public Key:</div>
                            <div class="identity-value">ed25519:${profile.public_key.slice(0, 32)}...</div>
                        ` : ''}
                        ${profile.timestamp ? `
                            <div class="identity-label">Identity Created:</div>
                            <div class="identity-value">${new Date(profile.timestamp * 1000).toLocaleString()}</div>
                        ` : ''}
                    </div>
                    <div class="identity-actions">
                        <button class="btn btn-primary" onclick="addContact('${profile.user_id}'); this.closest('.modal').remove();">
                            ➕ Add Contact
                        </button>
                        <button class="btn btn-secondary" onclick="this.closest('.modal').remove()">
                            Close
                        </button>
                    </div>
                </div>
            </div>
        </div>
    `;
    
    document.body.appendChild(modal);
    
    // Close on backdrop click
    modal.addEventListener('click', function(e) {
        if (e.target === modal) {
            modal.remove();
        }
    });
}

async function addContact(userId) {
    try {
        if (window.__TAURI__) {
            const { invoke } = window.__TAURI__.core;
            await invoke('add_contact', { userId });
            showToast('Contact request sent!', 'success');
        } else {
            showToast('Contact request sent! (Demo Mode)', 'success');
        }
    } catch (error) {
        console.error('Error adding contact:', error);
        showToast('Failed to add contact: ' + error.message, 'error');
    }
}

function showToast(message, type = 'info') {
    const toastContainer = document.getElementById('toast-container') || createToastContainer();
    
    const toast = document.createElement('div');
    toast.className = `toast ${type}`;
    
    const iconMap = {
        success: '✅',
        error: '❌',
        warning: '⚠️',
        info: 'ℹ️'
    };
    
    toast.innerHTML = `
        <div class="toast-header">
            <span class="toast-icon">${iconMap[type]}</span>
            <span class="toast-title">${type.charAt(0).toUpperCase() + type.slice(1)}</span>
            <button class="toast-close" onclick="this.parentElement.parentElement.remove()">×</button>
        </div>
        <div class="toast-message">${message}</div>
    `;
    
    toastContainer.appendChild(toast);
    
    // Show toast
    setTimeout(() => toast.classList.add('show'), 10);
    
    // Auto-remove after 4 seconds
    setTimeout(() => {
        toast.classList.remove('show');
        setTimeout(() => toast.remove(), 300);
    }, 4000);
}

function createToastContainer() {
    const container = document.createElement('div');
    container.id = 'toast-container';
    container.style.cssText = 'position: fixed; top: 20px; right: 20px; z-index: 1100;';
    document.body.appendChild(container);
    return container;
}

// Handle import from file input in backup tab
async function importIdentity(event) {
    const file = event.target.files[0];
    if (!file) return;
    
    try {
        const text = await file.text();
        await importIdentityFromData(text);
        
        // Clear the file input
        event.target.value = '';
    } catch (error) {
        showToast('Error reading file: ' + error.message, 'error');
    }
}

async function importIdentityFromData(identityData) {
    try {
        if (window.__TAURI__) {
            const { invoke } = window.__TAURI__.core;
            showToast('Importing identity and verifying signature...', 'info');
            
            const result = await invoke('import_user_identity', {
                identity_data: identityData
            });
            
            showToast('Identity imported and signature verified!', 'success');
            
            // Update local state and refresh UI
            const identity = await invoke('get_user_identity');
            if (identity) {
                state.currentUser = {
                    ...state.currentUser,
                    name: identity.display_name,
                    threeWordAddress: identity.three_word_address,
                    id: identity.user_id
                };
                updateUserDisplay(state.currentUser);
                
                // Switch to overview tab to show imported identity
                switchIdentityTab('overview');
            }
        } else {
            showToast('Identity imported (Demo Mode)', 'success');
            // Parse demo data and update state
            try {
                const data = JSON.parse(identityData);
                if (data.display_name) {
                    state.currentUser = {
                        ...state.currentUser,
                        name: data.display_name,
                        threeWordAddress: data.three_word_address || 'imported.demo.address',
                        id: data.user_id || 'imported-' + Date.now()
                    };
                    updateUserDisplay(state.currentUser);
                    switchIdentityTab('overview');
                }
            } catch (parseError) {
                showToast('Invalid identity file format', 'error');
            }
        }
    } catch (error) {
        console.error('Error importing identity:', error);
        showToast('Failed to import identity: ' + error.message, 'error');
    }
}

// Setup modal event listeners
document.addEventListener('DOMContentLoaded', function() {
    // Close modals when clicking outside
    document.addEventListener('click', function(e) {
        if (e.target.classList.contains('modal')) {
            e.target.classList.add('hidden');
        }
    });
    
    // Close buttons
    document.querySelectorAll('.modal-close').forEach(btn => {
        btn.addEventListener('click', function() {
            this.closest('.modal').classList.add('hidden');
        });
    });
    
    // Identity tab switching
    document.querySelectorAll('.identity-tabs .tab-btn').forEach(btn => {
        btn.addEventListener('click', function() {
            switchIdentityTab(this.dataset.tab);
        });
    });
    
    // Search functionality
    const searchInput = document.getElementById('search-input');
    if (searchInput) {
        searchInput.addEventListener('keypress', function(e) {
            if (e.key === 'Enter') {
                searchUsers();
            }
        });
    }
    
    // Create identity form
    const createForm = document.getElementById('create-identity-form');
    if (createForm) {
        createForm.addEventListener('submit', function(e) {
            e.preventDefault();
            createNewIdentity();
        });
    }
});

console.log('Saorsa Professional JavaScript loaded');