// src/passkey-auth.js - Frontend passkey authentication module

class PasskeyAuth {
    constructor() {
        this.isAvailable = false;
        this.hasPasskey = false;
        this.platformInfo = null;
    }
    
    async init() {
        try {
            // Check if running in Tauri
            if (!window.__TAURI__) {
                console.warn('Passkey auth requires Tauri environment');
                return;
            }
            
            const { invoke } = window.__TAURI__.core;
            
            // Check availability and get platform info
            [this.isAvailable, this.platformInfo] = await Promise.all([
                invoke('check_passkey_availability'),
                invoke('get_passkey_platform_info')
            ]);
            
            console.log('Passkey availability:', this.isAvailable);
            console.log('Platform info:', this.platformInfo);
            
            // Check if user has existing passkeys
            const credentials = await this.getStoredCredentials();
            this.hasPasskey = credentials && credentials.length > 0;
            
            console.log('Has existing passkeys:', this.hasPasskey);
        } catch (error) {
            console.error('Failed to initialize passkey auth:', error);
            this.isAvailable = false;
        }
    }
    
    async createPasskey(backupPassword) {
        if (!this.isAvailable) {
            throw new Error('Passkeys not available on this device');
        }
        
        if (!backupPassword || backupPassword.length < 8) {
            throw new Error('Backup password must be at least 8 characters');
        }
        
        try {
            const { invoke } = window.__TAURI__.core;
            const result = await invoke('create_passkey', {
                password: backupPassword
            });
            
            this.hasPasskey = true;
            console.log('Passkey created successfully:', result);
            return result;
        } catch (error) {
            console.error('Failed to create passkey:', error);
            throw error;
        }
    }
    
    async authenticate() {
        if (!this.hasPasskey) {
            throw new Error('No passkey found - please create one first');
        }
        
        try {
            const { invoke } = window.__TAURI__.core;
            const result = await invoke('authenticate_with_passkey');
            console.log('Passkey authentication successful:', result);
            return result;
        } catch (error) {
            console.error('Passkey authentication failed:', error);
            throw error;
        }
    }
    
    async authenticateWithBackup(threeWords, pin) {
        if (!threeWords || threeWords.length !== 3) {
            throw new Error('Must provide exactly 3 words');
        }
        
        if (!pin || pin.length < 4) {
            throw new Error('PIN must be at least 4 characters');
        }
        
        try {
            const { invoke } = window.__TAURI__.core;
            const result = await invoke('authenticate_with_three_words', {
                three_words: threeWords,
                pin: pin
            });
            console.log('Backup authentication successful:', result);
            return result;
        } catch (error) {
            console.error('Backup authentication failed:', error);
            throw error;
        }
    }
    
    async getStoredCredentials() {
        try {
            const { invoke } = window.__TAURI__.core;
            return await invoke('get_stored_passkey_credentials');
        } catch (error) {
            console.error('Failed to get credentials:', error);
            return [];
        }
    }
    
    // Helper methods
    getPlatformName() {
        if (!this.platformInfo) return 'Unknown';
        return this.platformInfo.platform;
    }
    
    getSupportedFeatures() {
        if (!this.platformInfo) return {};
        return this.platformInfo.supported_features;
    }
    
    canUseBiometrics() {
        return this.isAvailable && this.getSupportedFeatures().biometric_auth;
    }
}

// UI Components for passkey authentication
export class PasskeyUI {
    constructor(auth) {
        this.auth = auth;
        this.setupModalTemplate();
    }
    
    setupModalTemplate() {
        // Create reusable modal template
        this.modalTemplate = `
            <div class="passkey-modal-backdrop">
                <div class="passkey-modal-container">
                    <div class="passkey-modal-header">
                        <h2 class="passkey-modal-title"></h2>
                        <button class="passkey-modal-close" onclick="this.closest('.passkey-modal').remove()">
                            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                <line x1="18" y1="6" x2="6" y2="18"></line>
                                <line x1="6" y1="6" x2="18" y2="18"></line>
                            </svg>
                        </button>
                    </div>
                    <div class="passkey-modal-body"></div>
                    <div class="passkey-modal-footer"></div>
                </div>
            </div>
        `;
    }
    
    createModal(title, bodyContent, footerContent = '') {
        const modal = document.createElement('div');
        modal.className = 'passkey-modal';
        modal.innerHTML = this.modalTemplate;
        
        modal.querySelector('.passkey-modal-title').textContent = title;
        modal.querySelector('.passkey-modal-body').innerHTML = bodyContent;
        if (footerContent) {
            modal.querySelector('.passkey-modal-footer').innerHTML = footerContent;
        }
        
        return modal;
    }
    
    async showSetupFlow() {
        const setupContent = this.createSetupContent();
        const modal = this.createModal('Setup Secure Access', setupContent);
        
        document.body.appendChild(modal);
        this.bindSetupEvents(modal);
        
        return new Promise((resolve, reject) => {
            modal.addEventListener('setupComplete', (e) => {
                modal.remove();
                resolve(e.detail);
            });
            
            modal.addEventListener('setupCanceled', () => {
                modal.remove();
                reject(new Error('Setup canceled'));
            });
        });
    }
    
    createSetupContent() {
        const platformSupport = this.auth.canUseBiometrics();
        const platformName = this.auth.getPlatformName();
        
        return `
            <div class="setup-section">
                <div class="platform-info">
                    <div class="platform-status ${platformSupport ? 'supported' : 'unsupported'}">
                        <div class="platform-icon">
                            ${platformSupport ? '✅' : '❌'}
                        </div>
                        <div class="platform-details">
                            <div class="platform-name">${platformName}</div>
                            <div class="platform-description">
                                ${platformSupport 
                                    ? 'Biometric authentication is available' 
                                    : 'Biometric authentication is not available'}
                            </div>
                        </div>
                    </div>
                </div>
                
                ${platformSupport ? `
                    <div class="biometric-setup">
                        <h3>🔐 Biometric Authentication</h3>
                        <p>Use ${platformName} to unlock your data with biometric authentication.</p>
                        <div class="setup-instructions">
                            <ol>
                                <li>Create a backup password first</li>
                                <li>Enable biometric access</li>
                                <li>Your data will be encrypted and secured</li>
                            </ol>
                        </div>
                    </div>
                ` : ''}
                
                <div class="backup-setup">
                    <h3>🔑 Backup Access Method</h3>
                    <p>Create a backup method using three words and a PIN for secure access.</p>
                    
                    <div class="form-group">
                        <label class="form-label" for="setup-word1">Three Words</label>
                        <div class="three-words-input">
                            <input type="text" id="setup-word1" class="form-input" placeholder="First word" autocomplete="off" />
                            <input type="text" id="setup-word2" class="form-input" placeholder="Second word" autocomplete="off" />
                            <input type="text" id="setup-word3" class="form-input" placeholder="Third word" autocomplete="off" />
                        </div>
                        <div class="form-help">Choose three memorable words</div>
                    </div>
                    
                    <div class="form-group">
                        <label class="form-label" for="setup-pin">Backup PIN</label>
                        <input type="password" id="setup-pin" class="form-input" placeholder="Enter a secure PIN (8+ chars)" />
                        <div class="form-help">This PIN will be used for backup access and encryption</div>
                    </div>
                    
                    <div class="form-group">
                        <label class="form-label" for="setup-pin-confirm">Confirm PIN</label>
                        <input type="password" id="setup-pin-confirm" class="form-input" placeholder="Confirm your PIN" />
                    </div>
                    
                    <div class="setup-actions">
                        ${platformSupport ? `
                            <button class="btn btn-primary" id="setup-biometric">
                                🔐 Setup Biometric + Backup
                            </button>
                        ` : ''}
                        <button class="btn btn-secondary" id="setup-backup-only">
                            🔑 Setup Backup Only
                        </button>
                    </div>
                </div>
                
                <div id="setup-status" class="status-message hidden"></div>
            </div>
        `;
    }
    
    bindSetupEvents(modal) {
        const biometricBtn = modal.querySelector('#setup-biometric');
        const backupBtn = modal.querySelector('#setup-backup-only');
        const statusDiv = modal.querySelector('#setup-status');
        
        const handleSetup = async (includeBiometric) => {
            try {
                const words = [
                    modal.querySelector('#setup-word1').value.trim(),
                    modal.querySelector('#setup-word2').value.trim(),
                    modal.querySelector('#setup-word3').value.trim()
                ];
                const pin = modal.querySelector('#setup-pin').value;
                const confirmPin = modal.querySelector('#setup-pin-confirm').value;
                
                // Validation
                if (words.some(w => !w)) {
                    throw new Error('Please enter all three words');
                }
                
                if (!pin || pin.length < 8) {
                    throw new Error('PIN must be at least 8 characters');
                }
                
                if (pin !== confirmPin) {
                    throw new Error('PINs do not match');
                }
                
                this.showStatus(statusDiv, 'Setting up secure access...', 'info');
                
                let result = null;
                if (includeBiometric) {
                    // Create passkey with biometric
                    result = await this.auth.createPasskey(pin);
                    this.showStatus(statusDiv, '✅ Biometric access enabled!', 'success');
                } else {
                    // Just validate the backup method
                    result = { method: 'backup_only', words, pin_length: pin.length };
                    this.showStatus(statusDiv, '✅ Backup access configured!', 'success');
                }
                
                setTimeout(() => {
                    const event = new CustomEvent('setupComplete', { detail: result });
                    modal.dispatchEvent(event);
                }, 1500);
                
            } catch (error) {
                this.showStatus(statusDiv, `❌ ${error.message}`, 'error');
            }
        };
        
        biometricBtn?.addEventListener('click', () => handleSetup(true));
        backupBtn?.addEventListener('click', () => handleSetup(false));
    }
    
    async showUnlockFlow() {
        const unlockContent = this.createUnlockContent();
        const modal = this.createModal('Unlock Saorsa', unlockContent);
        
        document.body.appendChild(modal);
        this.bindUnlockEvents(modal);
        
        return new Promise((resolve, reject) => {
            modal.addEventListener('unlockSuccess', (e) => {
                modal.remove();
                resolve(e.detail);
            });
            
            modal.addEventListener('unlockCanceled', () => {
                modal.remove();
                reject(new Error('Unlock canceled'));
            });
        });
    }
    
    createUnlockContent() {
        return `
            <div class="unlock-section">
                ${this.auth.hasPasskey ? `
                    <div class="biometric-unlock">
                        <div class="unlock-method primary">
                            <div class="method-icon">🔐</div>
                            <div class="method-info">
                                <h3>Biometric Authentication</h3>
                                <p>Use ${this.auth.getPlatformName()} to unlock your data</p>
                            </div>
                            <button class="btn btn-primary" id="unlock-biometric">
                                Unlock with Biometrics
                            </button>
                        </div>
                    </div>
                    
                    <div class="unlock-divider">
                        <span>or</span>
                    </div>
                ` : ''}
                
                <div class="backup-unlock">
                    <div class="unlock-method secondary">
                        <div class="method-icon">🔑</div>
                        <div class="method-info">
                            <h3>Backup Access</h3>
                            <p>Enter your three words and PIN</p>
                        </div>
                    </div>
                    
                    <div class="form-group">
                        <label class="form-label">Three Words</label>
                        <div class="three-words-input">
                            <input type="text" id="unlock-word1" class="form-input" placeholder="First word" autocomplete="off" />
                            <input type="text" id="unlock-word2" class="form-input" placeholder="Second word" autocomplete="off" />
                            <input type="text" id="unlock-word3" class="form-input" placeholder="Third word" autocomplete="off" />
                        </div>
                    </div>
                    
                    <div class="form-group">
                        <label class="form-label" for="unlock-pin">PIN</label>
                        <input type="password" id="unlock-pin" class="form-input" placeholder="Enter your PIN" />
                    </div>
                    
                    <button class="btn btn-secondary btn-full" id="unlock-backup">
                        Unlock with Backup
                    </button>
                </div>
                
                <div id="unlock-status" class="status-message hidden"></div>
            </div>
        `;
    }
    
    bindUnlockEvents(modal) {
        const biometricBtn = modal.querySelector('#unlock-biometric');
        const backupBtn = modal.querySelector('#unlock-backup');
        const statusDiv = modal.querySelector('#unlock-status');
        
        // Biometric unlock
        biometricBtn?.addEventListener('click', async () => {
            try {
                this.showStatus(statusDiv, 'Authenticating...', 'info');
                const result = await this.auth.authenticate();
                this.showStatus(statusDiv, '✅ Unlocked with biometrics!', 'success');
                
                setTimeout(() => {
                    const event = new CustomEvent('unlockSuccess', { detail: result });
                    modal.dispatchEvent(event);
                }, 1000);
                
            } catch (error) {
                this.showStatus(statusDiv, `❌ ${error.message}`, 'error');
                // Show backup form on failure
                modal.querySelector('.backup-unlock').style.display = 'block';
            }
        });
        
        // Backup unlock
        backupBtn?.addEventListener('click', async () => {
            try {
                const words = [
                    modal.querySelector('#unlock-word1').value.trim(),
                    modal.querySelector('#unlock-word2').value.trim(),
                    modal.querySelector('#unlock-word3').value.trim()
                ];
                const pin = modal.querySelector('#unlock-pin').value;
                
                if (words.some(w => !w) || !pin) {
                    throw new Error('Please enter all words and PIN');
                }
                
                this.showStatus(statusDiv, 'Verifying backup access...', 'info');
                const result = await this.auth.authenticateWithBackup(words, pin);
                this.showStatus(statusDiv, '✅ Unlocked with backup!', 'success');
                
                setTimeout(() => {
                    const event = new CustomEvent('unlockSuccess', { detail: result });
                    modal.dispatchEvent(event);
                }, 1000);
                
            } catch (error) {
                this.showStatus(statusDiv, `❌ ${error.message}`, 'error');
            }
        });
        
        // Auto-focus on first input
        const firstInput = modal.querySelector('#unlock-word1');
        if (firstInput) {
            setTimeout(() => firstInput.focus(), 100);
        }
    }
    
    showStatus(element, message, type) {
        element.className = `status-message ${type}`;
        element.textContent = message;
        element.classList.remove('hidden');
    }
    
    // Helper method to show simple notifications
    static showNotification(message, type = 'info', duration = 3000) {
        const notification = document.createElement('div');
        notification.className = `passkey-notification ${type}`;
        notification.innerHTML = `
            <div class="notification-content">
                <span class="notification-icon">
                    ${type === 'success' ? '✅' : type === 'error' ? '❌' : 'ℹ️'}
                </span>
                <span class="notification-message">${message}</span>
            </div>
        `;
        
        document.body.appendChild(notification);
        
        // Fade in
        setTimeout(() => notification.classList.add('show'), 10);
        
        // Fade out and remove
        setTimeout(() => {
            notification.classList.remove('show');
            setTimeout(() => notification.remove(), 300);
        }, duration);
    }
}

// Auto-initialize if in Tauri environment
let passkeyAuth = null;
let passkeyUI = null;

if (typeof window !== 'undefined' && window.__TAURI__) {
    document.addEventListener('DOMContentLoaded', async () => {
        passkeyAuth = new PasskeyAuth();
        passkeyUI = new PasskeyUI(passkeyAuth);
        
        try {
            await passkeyAuth.init();
            console.log('Passkey authentication system initialized');
            
            // Make available globally for testing
            window.saorsa = window.saorsa || {};
            window.saorsa.passkey = { auth: passkeyAuth, ui: passkeyUI };
        } catch (error) {
            console.error('Failed to initialize passkey system:', error);
        }
    });
}

// Global variables are available through window.saorsa.passkey