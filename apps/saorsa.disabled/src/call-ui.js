// Call UI components for Saorsa
import { WebRTCManager, CallState } from './webrtc.js';

class CallUI {
    constructor() {
        this.activeCallUI = null;
        this.incomingCallDialog = null;
        this.callNotificationSound = new Audio('data:audio/wav;base64,UklGRnoGAABXQVZFZm10IBAAAAABAAEAQB8AAEAfAAABAAgAZGF0YQoGAACBhYqFbF');
        
        // Set up WebRTC event handlers
        this.setupWebRTCHandlers();
    }
    
    setupWebRTCHandlers() {
        const rtc = window.webRTCManager;
        
        rtc.onCallStateChange = (userId, state) => {
            this.updateCallState(userId, state);
        };
        
        rtc.onRemoteStream = (userId, stream) => {
            this.setRemoteStream(userId, stream);
        };
        
        rtc.onCallEnded = (userId, reason) => {
            this.handleCallEnded(userId, reason);
        };
        
        rtc.onCallError = (userId, error) => {
            this.showError(`Call error: ${error.message}`);
        };
    }
    
    // Show incoming call dialog
    showIncomingCall(userId, userName, channelName, isVideo) {
        // Play ringtone
        this.callNotificationSound.loop = true;
        this.callNotificationSound.play();
        
        // Create incoming call dialog
        this.incomingCallDialog = document.createElement('div');
        this.incomingCallDialog.className = 'incoming-call-dialog';
        this.incomingCallDialog.innerHTML = `
            <div class="incoming-call-content">
                <div class="caller-info">
                    <div class="caller-avatar">
                        <div class="avatar-placeholder">${this.getInitials(userName)}</div>
                    </div>
                    <h3 class="caller-name">${userName}</h3>
                    <p class="call-type">${isVideo ? 'Video' : 'Voice'} call in ${channelName}</p>
                </div>
                
                <div class="call-actions">
                    <button class="decline-btn" onclick="callUI.declineCall('${userId}')">
                        <svg width="24" height="24" viewBox="0 0 24 24">
                            <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm5 11H7v-2h10v2z" fill="currentColor"/>
                        </svg>
                        Decline
                    </button>
                    <button class="accept-btn" onclick="callUI.acceptCall('${userId}')">
                        <svg width="24" height="24" viewBox="0 0 24 24">
                            <path d="M20 15.5c-1.25 0-2.45-.2-3.57-.57-.35-.11-.74-.03-1.02.24l-2.2 2.2c-2.83-1.44-5.15-3.75-6.59-6.59l2.2-2.21c.28-.26.36-.65.25-1C8.7 6.45 8.5 5.25 8.5 4c0-.55-.45-1-1-1H4c-.55 0-1 .45-1 1 0 9.39 7.61 17 17 17 .55 0 1-.45 1-1v-3.5c0-.55-.45-1-1-1z" fill="currentColor"/>
                        </svg>
                        Accept
                    </button>
                </div>
            </div>
        `;
        
        document.body.appendChild(this.incomingCallDialog);
    }
    
    // Accept incoming call
    async acceptCall(userId) {
        this.callNotificationSound.pause();
        this.callNotificationSound.currentTime = 0;
        
        if (this.incomingCallDialog) {
            this.incomingCallDialog.remove();
            this.incomingCallDialog = null;
        }
        
        try {
            await window.webRTCManager.acceptCall(userId);
            this.showActiveCall(userId);
        } catch (error) {
            this.showError(`Failed to accept call: ${error.message}`);
        }
    }
    
    // Decline incoming call
    async declineCall(userId) {
        this.callNotificationSound.pause();
        this.callNotificationSound.currentTime = 0;
        
        if (this.incomingCallDialog) {
            this.incomingCallDialog.remove();
            this.incomingCallDialog = null;
        }
        
        await window.webRTCManager.declineCall(userId);
    }
    
    // Show active call UI
    showActiveCall(userId, userName, isVideo) {
        if (this.activeCallUI) {
            this.activeCallUI.remove();
        }
        
        this.activeCallUI = document.createElement('div');
        this.activeCallUI.className = `active-call ${isVideo ? 'video-call' : 'voice-call'}`;
        this.activeCallUI.innerHTML = `
            <div class="call-container">
                ${isVideo ? `
                    <div class="video-grid">
                        <div class="remote-video-container">
                            <video id="remote-video" autoplay playsinline></video>
                            <div class="participant-info">
                                <span class="participant-name">${userName}</span>
                                <span class="connection-status">Connecting...</span>
                            </div>
                        </div>
                        <div class="local-video-container">
                            <video id="local-video" autoplay playsinline muted></video>
                        </div>
                    </div>
                ` : `
                    <div class="voice-call-ui">
                        <div class="participant-avatar">
                            <div class="avatar-placeholder large">${this.getInitials(userName)}</div>
                            <div class="audio-indicator">
                                <span class="audio-bar"></span>
                                <span class="audio-bar"></span>
                                <span class="audio-bar"></span>
                            </div>
                        </div>
                        <h2 class="participant-name">${userName}</h2>
                        <p class="call-duration">00:00</p>
                    </div>
                `}
                
                <div class="call-controls">
                    <button class="control-btn mute-btn" onclick="callUI.toggleMute('${userId}')" title="Toggle mute">
                        <svg width="24" height="24" viewBox="0 0 24 24">
                            <path d="M12 14c1.66 0 3-1.34 3-3V5c0-1.66-1.34-3-3-3S9 3.34 9 5v6c0 1.66 1.34 3 3 3zm-1 1.93c-3.94-.49-7-3.85-7-7.93 0-.41.34-.75.75-.75s.75.34.75.75c0 3.54 2.88 6.42 6.42 6.42s6.42-2.88 6.42-6.42c0-.41.34-.75.75-.75s.75.34.75.75c0 4.08-3.06 7.44-7 7.93V20h3c.41 0 .75.34.75.75s-.34.75-.75.75h-7.5c-.41 0-.75-.34-.75-.75s.34-.75.75-.75h3v-4.07z" fill="currentColor"/>
                        </svg>
                    </button>
                    
                    ${isVideo ? `
                        <button class="control-btn video-btn" onclick="callUI.toggleVideo('${userId}')" title="Toggle video">
                            <svg width="24" height="24" viewBox="0 0 24 24">
                                <path d="M17 10.5V7c0-.55-.45-1-1-1H4c-.55 0-1 .45-1 1v10c0 .55.45 1 1 1h12c.55 0 1-.45 1-1v-3.5l4 4v-11l-4 4z" fill="currentColor"/>
                            </svg>
                        </button>
                    ` : ''}
                    
                    <button class="control-btn end-call-btn" onclick="callUI.endCall('${userId}')" title="End call">
                        <svg width="24" height="24" viewBox="0 0 24 24">
                            <path d="M12 9c-1.6 0-3.15.25-4.6.72v3.1c0 .39-.23.74-.56.9-.98.49-1.87 1.12-2.66 1.85-.18.18-.43.28-.68.28-.3 0-.56-.13-.75-.33l-2.2-2.2c-.2-.2-.33-.46-.33-.76s.13-.56.33-.75c1.46-1.45 3.31-2.59 5.42-3.31 1.62-.56 3.31-.84 5.03-.84s3.41.28 5.03.84c2.11.73 3.96 1.86 5.42 3.31.2.2.33.46.33.75s-.13.56-.33.76l-2.2 2.2c-.19.2-.45.33-.75.33-.25 0-.5-.1-.68-.28-.79-.74-1.69-1.36-2.67-1.85-.33-.16-.56-.5-.56-.9v-3.1c-1.45-.48-3-.73-4.6-.73z" fill="white"/>
                        </svg>
                    </button>
                    
                    <button class="control-btn more-btn" onclick="callUI.showMoreOptions('${userId}')" title="More options">
                        <svg width="24" height="24" viewBox="0 0 24 24">
                            <path d="M12 8c1.1 0 2-.9 2-2s-.9-2-2-2-2 .9-2 2 .9 2 2 2zm0 2c-1.1 0-2 .9-2 2s.9 2 2 2 2-.9 2-2-.9-2-2-2zm0 6c-1.1 0-2 .9-2 2s.9 2 2 2 2-.9 2-2-.9-2-2-2z" fill="currentColor"/>
                        </svg>
                    </button>
                </div>
                
                <div class="call-quality-indicator">
                    <span class="quality-bars">
                        <span class="bar active"></span>
                        <span class="bar active"></span>
                        <span class="bar active"></span>
                        <span class="bar"></span>
                    </span>
                    <span class="quality-text">Good connection</span>
                </div>
            </div>
        `;
        
        document.body.appendChild(this.activeCallUI);
        
        // Set up local video if it's a video call
        if (isVideo) {
            const localVideo = document.getElementById('local-video');
            if (localVideo && window.webRTCManager.localStream) {
                localVideo.srcObject = window.webRTCManager.localStream;
            }
        }
        
        // Start call duration timer
        this.startCallTimer();
        
        // Start monitoring call quality
        this.startQualityMonitoring(userId);
    }
    
    // Set remote stream
    setRemoteStream(userId, stream) {
        const remoteVideo = document.getElementById('remote-video');
        if (remoteVideo) {
            remoteVideo.srcObject = stream;
        }
        
        // Update connection status
        const statusEl = this.activeCallUI?.querySelector('.connection-status');
        if (statusEl) {
            statusEl.textContent = 'Connected';
            statusEl.classList.add('connected');
        }
    }
    
    // Toggle mute
    async toggleMute(userId) {
        const isEnabled = window.webRTCManager.toggleAudio(userId);
        const muteBtn = this.activeCallUI?.querySelector('.mute-btn');
        
        if (muteBtn) {
            muteBtn.classList.toggle('muted', !isEnabled);
            muteBtn.innerHTML = isEnabled ? `
                <svg width="24" height="24" viewBox="0 0 24 24">
                    <path d="M12 14c1.66 0 3-1.34 3-3V5c0-1.66-1.34-3-3-3S9 3.34 9 5v6c0 1.66 1.34 3 3 3zm-1 1.93c-3.94-.49-7-3.85-7-7.93 0-.41.34-.75.75-.75s.75.34.75.75c0 3.54 2.88 6.42 6.42 6.42s6.42-2.88 6.42-6.42c0-.41.34-.75.75-.75s.75.34.75.75c0 4.08-3.06 7.44-7 7.93V20h3c.41 0 .75.34.75.75s-.34.75-.75.75h-7.5c-.41 0-.75-.34-.75-.75s.34-.75.75-.75h3v-4.07z" fill="currentColor"/>
                </svg>
            ` : `
                <svg width="24" height="24" viewBox="0 0 24 24">
                    <path d="M19 11h-1.7c0 .74-.16 1.43-.43 2.05l1.23 1.23c.56-.98.9-2.09.9-3.28zm-4.02.17c0-.06.02-.11.02-.17V5c0-1.66-1.34-3-3-3S9 3.34 9 5v.18l5.98 5.99zM4.27 3L3 4.27l6.01 6.01V11c0 1.66 1.33 3 2.99 3 .22 0 .44-.03.65-.08l1.66 1.66c-.71.33-1.5.52-2.31.52-2.76 0-5.3-2.1-5.3-5.1H5c0 3.41 2.72 6.23 6 6.72V21h2v-3.28c.91-.13 1.77-.45 2.54-.9L19.73 21 21 19.73 4.27 3z" fill="currentColor"/>
                </svg>
            `;
        }
    }
    
    // Toggle video
    async toggleVideo(userId) {
        const isEnabled = await window.webRTCManager.toggleVideo(userId);
        const videoBtn = this.activeCallUI?.querySelector('.video-btn');
        
        if (videoBtn) {
            videoBtn.classList.toggle('disabled', !isEnabled);
        }
        
        // Show/hide local video
        const localVideo = document.getElementById('local-video');
        if (localVideo) {
            localVideo.style.display = isEnabled ? 'block' : 'none';
        }
    }
    
    // End call
    async endCall(userId) {
        await window.webRTCManager.endCall(userId);
        
        if (this.activeCallUI) {
            this.activeCallUI.remove();
            this.activeCallUI = null;
        }
        
        this.stopCallTimer();
        this.stopQualityMonitoring();
    }
    
    // Handle call ended
    handleCallEnded(userId, reason) {
        if (this.activeCallUI) {
            this.activeCallUI.remove();
            this.activeCallUI = null;
        }
        
        this.stopCallTimer();
        this.stopQualityMonitoring();
        
        // Show notification
        const message = reason === 'declined' ? 'Call declined' : 
                       reason === 'connection-failed' ? 'Call failed' : 
                       'Call ended';
        
        window.SaorsaPro?.showNotification(message, 'info');
    }
    
    // Update call state in UI
    updateCallState(userId, state) {
        const statusEl = this.activeCallUI?.querySelector('.connection-status');
        if (statusEl) {
            switch (state) {
                case CallState.CALLING:
                    statusEl.textContent = 'Calling...';
                    break;
                case CallState.CONNECTING:
                    statusEl.textContent = 'Connecting...';
                    break;
                case CallState.CONNECTED:
                    statusEl.textContent = 'Connected';
                    statusEl.classList.add('connected');
                    break;
                case CallState.FAILED:
                    statusEl.textContent = 'Connection failed';
                    statusEl.classList.add('failed');
                    break;
            }
        }
    }
    
    // Call timer
    startCallTimer() {
        let seconds = 0;
        this.callTimer = setInterval(() => {
            seconds++;
            const minutes = Math.floor(seconds / 60);
            const secs = seconds % 60;
            const duration = `${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
            
            const durationEl = this.activeCallUI?.querySelector('.call-duration');
            if (durationEl) {
                durationEl.textContent = duration;
            }
        }, 1000);
    }
    
    stopCallTimer() {
        if (this.callTimer) {
            clearInterval(this.callTimer);
            this.callTimer = null;
        }
    }
    
    // Quality monitoring
    startQualityMonitoring(userId) {
        this.qualityInterval = setInterval(async () => {
            const stats = await window.webRTCManager.getCallStats(userId);
            if (stats) {
                this.updateQualityIndicator(stats);
            }
        }, 2000);
    }
    
    stopQualityMonitoring() {
        if (this.qualityInterval) {
            clearInterval(this.qualityInterval);
            this.qualityInterval = null;
        }
    }
    
    updateQualityIndicator(stats) {
        const bars = this.activeCallUI?.querySelectorAll('.quality-bars .bar');
        const text = this.activeCallUI?.querySelector('.quality-text');
        
        if (!bars || !text) return;
        
        // Calculate quality based on bitrate and packet loss
        let quality = 4;
        if (stats.audio.packetsLost > 5 || stats.audio.bitrate < 20) {
            quality = 1;
        } else if (stats.audio.packetsLost > 2 || stats.audio.bitrate < 40) {
            quality = 2;
        } else if (stats.audio.packetsLost > 0 || stats.audio.bitrate < 60) {
            quality = 3;
        }
        
        // Update bars
        bars.forEach((bar, index) => {
            bar.classList.toggle('active', index < quality);
        });
        
        // Update text
        const qualityTexts = ['Poor connection', 'Fair connection', 'Good connection', 'Excellent connection'];
        text.textContent = qualityTexts[quality - 1];
    }
    
    // Show more options menu
    showMoreOptions(userId) {
        // Implementation for additional options like screen sharing, recording, etc.
    }
    
    // Utility functions
    getInitials(name) {
        return name
            .split(' ')
            .map(n => n[0])
            .join('')
            .toUpperCase()
            .slice(0, 2);
    }
    
    showError(message) {
        window.SaorsaPro?.showNotification(message, 'error');
    }
}

// Create global instance
window.callUI = new CallUI();

export { CallUI };