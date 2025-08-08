// WebRTC module for voice and video calls in Saorsa
import { invoke } from '@tauri-apps/api/tauri';

// WebRTC configuration with STUN/TURN servers
const rtcConfig = {
    iceServers: [
        { urls: 'stun:stun.l.google.com:19302' },
        { urls: 'stun:stun1.l.google.com:19302' },
        // Add TURN servers for better connectivity through NATs
        // These would be configured based on your deployment
    ],
    iceCandidatePoolSize: 10
};

// Media constraints
const mediaConstraints = {
    audio: {
        echoCancellation: true,
        noiseSuppression: true,
        autoGainControl: true,
        sampleRate: 48000
    },
    video: {
        width: { min: 640, ideal: 1280, max: 1920 },
        height: { min: 480, ideal: 720, max: 1080 },
        frameRate: { ideal: 30, max: 60 },
        facingMode: 'user'
    }
};

// Call states
const CallState = {
    IDLE: 'idle',
    CALLING: 'calling',
    INCOMING: 'incoming',
    CONNECTING: 'connecting',
    CONNECTED: 'connected',
    ENDED: 'ended',
    FAILED: 'failed'
};

class WebRTCManager {
    constructor() {
        this.calls = new Map(); // Map of userId -> Call
        this.localStream = null;
        this.audioContext = null;
        this.audioAnalyser = null;
        this.videoStatsInterval = null;
        
        // Event handlers
        this.onCallStateChange = null;
        this.onRemoteStream = null;
        this.onCallEnded = null;
        this.onCallError = null;
        
        // Initialize audio context for audio level detection
        this.initAudioContext();
    }
    
    initAudioContext() {
        this.audioContext = new (window.AudioContext || window.webkitAudioContext)();
        this.audioAnalyser = this.audioContext.createAnalyser();
        this.audioAnalyser.fftSize = 256;
        this.audioAnalyser.smoothingTimeConstant = 0.8;
    }
    
    // Initialize media devices
    async initializeMedia(audio = true, video = false) {
        try {
            const constraints = {
                audio: audio ? mediaConstraints.audio : false,
                video: video ? mediaConstraints.video : false
            };
            
            this.localStream = await navigator.mediaDevices.getUserMedia(constraints);
            
            // Connect audio to analyser for level detection
            if (audio && this.localStream.getAudioTracks().length > 0) {
                const source = this.audioContext.createMediaStreamSource(this.localStream);
                source.connect(this.audioAnalyser);
            }
            
            return this.localStream;
        } catch (error) {
            console.error('Failed to get user media:', error);
            throw new Error(`Media access failed: ${error.message}`);
        }
    }
    
    // Start a voice or video call
    async startCall(userId, channelId, isVideo = false) {
        try {
            // Initialize media
            await this.initializeMedia(true, isVideo);
            
            // Create new call
            const call = new Call(userId, channelId, true, isVideo);
            this.calls.set(userId, call);
            
            // Set up peer connection
            await call.setupPeerConnection(this.localStream);
            
            // Create offer
            const offer = await call.createOffer();
            
            // Send offer through P2P network
            await invoke('send_call_offer', {
                userId,
                channelId,
                offer: offer.sdp,
                isVideo
            });
            
            call.state = CallState.CALLING;
            this.onCallStateChange?.(userId, CallState.CALLING);
            
            return call;
        } catch (error) {
            console.error('Failed to start call:', error);
            this.onCallError?.(userId, error);
            throw error;
        }
    }
    
    // Handle incoming call offer
    async handleIncomingCall(userId, channelId, offerSdp, isVideo) {
        try {
            // Check if we already have a call with this user
            if (this.calls.has(userId)) {
                console.warn('Already in call with user:', userId);
                return;
            }
            
            // Create new call
            const call = new Call(userId, channelId, false, isVideo);
            this.calls.set(userId, call);
            
            call.state = CallState.INCOMING;
            this.onCallStateChange?.(userId, CallState.INCOMING);
            
            // Store the offer for when user accepts
            call.pendingOffer = offerSdp;
            
            return call;
        } catch (error) {
            console.error('Failed to handle incoming call:', error);
            this.onCallError?.(userId, error);
            throw error;
        }
    }
    
    // Accept an incoming call
    async acceptCall(userId) {
        const call = this.calls.get(userId);
        if (!call || call.state !== CallState.INCOMING) {
            throw new Error('No incoming call from this user');
        }
        
        try {
            // Initialize media
            await this.initializeMedia(true, call.isVideo);
            
            // Set up peer connection
            await call.setupPeerConnection(this.localStream);
            
            // Set remote offer
            await call.setRemoteOffer(call.pendingOffer);
            
            // Create answer
            const answer = await call.createAnswer();
            
            // Send answer through P2P network
            await invoke('send_call_answer', {
                userId,
                channelId: call.channelId,
                answer: answer.sdp
            });
            
            call.state = CallState.CONNECTING;
            this.onCallStateChange?.(userId, CallState.CONNECTING);
        } catch (error) {
            console.error('Failed to accept call:', error);
            this.onCallError?.(userId, error);
            throw error;
        }
    }
    
    // Decline an incoming call
    async declineCall(userId) {
        const call = this.calls.get(userId);
        if (!call || call.state !== CallState.INCOMING) {
            return;
        }
        
        await this.endCall(userId, 'declined');
    }
    
    // End a call
    async endCall(userId, reason = 'user-ended') {
        const call = this.calls.get(userId);
        if (!call) return;
        
        try {
            // Close peer connection
            call.close();
            
            // Remove from active calls
            this.calls.delete(userId);
            
            // Notify remote peer
            await invoke('end_call', {
                userId,
                channelId: call.channelId,
                reason
            });
            
            // Stop local stream if no other calls
            if (this.calls.size === 0 && this.localStream) {
                this.localStream.getTracks().forEach(track => track.stop());
                this.localStream = null;
            }
            
            this.onCallEnded?.(userId, reason);
        } catch (error) {
            console.error('Failed to end call:', error);
        }
    }
    
    // Toggle audio mute
    toggleAudio(userId) {
        const call = this.calls.get(userId);
        if (!call) return false;
        
        const audioTrack = this.localStream?.getAudioTracks()[0];
        if (audioTrack) {
            audioTrack.enabled = !audioTrack.enabled;
            return audioTrack.enabled;
        }
        return false;
    }
    
    // Toggle video
    async toggleVideo(userId) {
        const call = this.calls.get(userId);
        if (!call) return false;
        
        const videoTrack = this.localStream?.getVideoTracks()[0];
        
        if (videoTrack) {
            // Disable existing video
            videoTrack.enabled = !videoTrack.enabled;
            return videoTrack.enabled;
        } else if (call.isVideo) {
            // Add video track if it's a video call
            try {
                const videoStream = await navigator.mediaDevices.getUserMedia({
                    video: mediaConstraints.video
                });
                const newVideoTrack = videoStream.getVideoTracks()[0];
                
                // Add to local stream
                this.localStream.addTrack(newVideoTrack);
                
                // Add to peer connection
                const sender = call.pc.getSenders().find(
                    s => s.track && s.track.kind === 'video'
                );
                if (sender) {
                    sender.replaceTrack(newVideoTrack);
                } else {
                    call.pc.addTrack(newVideoTrack, this.localStream);
                }
                
                return true;
            } catch (error) {
                console.error('Failed to add video:', error);
                return false;
            }
        }
        
        return false;
    }
    
    // Get audio levels for UI visualization
    getAudioLevel() {
        if (!this.audioAnalyser) return 0;
        
        const dataArray = new Uint8Array(this.audioAnalyser.frequencyBinCount);
        this.audioAnalyser.getByteFrequencyData(dataArray);
        
        // Calculate average volume
        const average = dataArray.reduce((a, b) => a + b, 0) / dataArray.length;
        return average / 255; // Normalize to 0-1
    }
    
    // Get call statistics
    async getCallStats(userId) {
        const call = this.calls.get(userId);
        if (!call || !call.pc) return null;
        
        const stats = await call.pc.getStats();
        const result = {
            audio: { bitrate: 0, packetsLost: 0, jitter: 0 },
            video: { bitrate: 0, packetsLost: 0, frameRate: 0, resolution: '' }
        };
        
        stats.forEach(report => {
            if (report.type === 'inbound-rtp') {
                if (report.mediaType === 'audio') {
                    result.audio.bitrate = report.bytesReceived * 8 / 1000; // kbps
                    result.audio.packetsLost = report.packetsLost || 0;
                    result.audio.jitter = report.jitter || 0;
                } else if (report.mediaType === 'video') {
                    result.video.bitrate = report.bytesReceived * 8 / 1000; // kbps
                    result.video.packetsLost = report.packetsLost || 0;
                    result.video.frameRate = report.framesPerSecond || 0;
                    if (report.frameWidth && report.frameHeight) {
                        result.video.resolution = `${report.frameWidth}x${report.frameHeight}`;
                    }
                }
            }
        });
        
        return result;
    }
}

// Individual call handler
class Call {
    constructor(userId, channelId, isInitiator, isVideo) {
        this.userId = userId;
        this.channelId = channelId;
        this.isInitiator = isInitiator;
        this.isVideo = isVideo;
        this.state = CallState.IDLE;
        this.pc = null;
        this.remoteStream = null;
        this.pendingOffer = null;
        this.iceCandidates = [];
        this.dataChannel = null;
    }
    
    async setupPeerConnection(localStream) {
        this.pc = new RTCPeerConnection(rtcConfig);
        
        // Add local stream tracks
        localStream.getTracks().forEach(track => {
            this.pc.addTrack(track, localStream);
        });
        
        // Handle remote stream
        this.pc.ontrack = (event) => {
            if (!this.remoteStream) {
                this.remoteStream = new MediaStream();
            }
            this.remoteStream.addTrack(event.track);
            
            // Notify manager
            window.webRTCManager.onRemoteStream?.(this.userId, this.remoteStream);
        };
        
        // Handle ICE candidates
        this.pc.onicecandidate = async (event) => {
            if (event.candidate) {
                await invoke('send_ice_candidate', {
                    userId: this.userId,
                    candidate: event.candidate.toJSON()
                });
            }
        };
        
        // Handle connection state changes
        this.pc.onconnectionstatechange = () => {
            console.log('Connection state:', this.pc.connectionState);
            
            switch (this.pc.connectionState) {
                case 'connected':
                    this.state = CallState.CONNECTED;
                    window.webRTCManager.onCallStateChange?.(this.userId, CallState.CONNECTED);
                    break;
                case 'failed':
                case 'disconnected':
                    this.state = CallState.FAILED;
                    window.webRTCManager.onCallStateChange?.(this.userId, CallState.FAILED);
                    window.webRTCManager.endCall(this.userId, 'connection-failed');
                    break;
            }
        };
        
        // Set up data channel for call metadata
        if (this.isInitiator) {
            this.dataChannel = this.pc.createDataChannel('call-metadata', {
                ordered: true
            });
            this.setupDataChannel();
        } else {
            this.pc.ondatachannel = (event) => {
                this.dataChannel = event.channel;
                this.setupDataChannel();
            };
        }
    }
    
    setupDataChannel() {
        this.dataChannel.onopen = () => {
            console.log('Data channel opened');
        };
        
        this.dataChannel.onmessage = (event) => {
            try {
                const data = JSON.parse(event.data);
                this.handleDataChannelMessage(data);
            } catch (error) {
                console.error('Failed to parse data channel message:', error);
            }
        };
    }
    
    handleDataChannelMessage(data) {
        switch (data.type) {
            case 'mute-status':
                // Handle remote mute status
                break;
            case 'screen-share':
                // Handle screen share status
                break;
            case 'call-quality':
                // Handle quality adjustment
                break;
        }
    }
    
    async createOffer() {
        const offer = await this.pc.createOffer({
            offerToReceiveAudio: true,
            offerToReceiveVideo: this.isVideo
        });
        await this.pc.setLocalDescription(offer);
        return offer;
    }
    
    async createAnswer() {
        const answer = await this.pc.createAnswer();
        await this.pc.setLocalDescription(answer);
        return answer;
    }
    
    async setRemoteOffer(offerSdp) {
        const offer = new RTCSessionDescription({
            type: 'offer',
            sdp: offerSdp
        });
        await this.pc.setRemoteDescription(offer);
    }
    
    async setRemoteAnswer(answerSdp) {
        const answer = new RTCSessionDescription({
            type: 'answer',
            sdp: answerSdp
        });
        await this.pc.setRemoteDescription(answer);
    }
    
    async addIceCandidate(candidate) {
        if (this.pc.remoteDescription) {
            await this.pc.addIceCandidate(new RTCIceCandidate(candidate));
        } else {
            // Queue candidates if remote description not set yet
            this.iceCandidates.push(candidate);
        }
    }
    
    async processQueuedCandidates() {
        for (const candidate of this.iceCandidates) {
            await this.pc.addIceCandidate(new RTCIceCandidate(candidate));
        }
        this.iceCandidates = [];
    }
    
    sendDataChannelMessage(data) {
        if (this.dataChannel && this.dataChannel.readyState === 'open') {
            this.dataChannel.send(JSON.stringify(data));
        }
    }
    
    close() {
        if (this.dataChannel) {
            this.dataChannel.close();
        }
        if (this.pc) {
            this.pc.close();
        }
        if (this.remoteStream) {
            this.remoteStream.getTracks().forEach(track => track.stop());
        }
    }
}

// Create global instance
window.webRTCManager = new WebRTCManager();

// Export for use in other modules
export { WebRTCManager, CallState };