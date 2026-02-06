// Dashboard JavaScript - voice recording and transcription
import { iconMicrophone } from '../icons';

export const voiceJs = `
        // Voice recording state per session
        const voiceRecorders = new Map(); // sessionId -> VoiceRecorder

        class VoiceRecorder {
            constructor() {
                this.mediaRecorder = null;
                this.audioContext = null;
                this.analyser = null;
                this.stream = null;
                this.chunks = [];
                this.isRecording = false;
                this.levelInterval = null;
            }

            static isSupported() {
                return !!(navigator.mediaDevices && navigator.mediaDevices.getUserMedia && window.MediaRecorder);
            }

            async startRecording() {
                this.stream = await navigator.mediaDevices.getUserMedia({ audio: true });

                // Try codecs in order of preference
                const mimeTypes = ['audio/webm;codecs=opus', 'audio/webm', 'audio/ogg;codecs=opus', 'audio/ogg'];
                let mimeType = '';
                for (const mt of mimeTypes) {
                    if (MediaRecorder.isTypeSupported(mt)) {
                        mimeType = mt;
                        break;
                    }
                }

                this.chunks = [];
                this.mediaRecorder = new MediaRecorder(this.stream, mimeType ? { mimeType } : {});
                this.mediaRecorder.ondataavailable = (e) => {
                    if (e.data.size > 0) this.chunks.push(e.data);
                };

                this.mediaRecorder.start();
                this.isRecording = true;
                this.setupAudioLevelMonitor();
            }

            setupAudioLevelMonitor() {
                this.audioContext = new AudioContext();
                const source = this.audioContext.createMediaStreamSource(this.stream);
                this.analyser = this.audioContext.createAnalyser();
                this.analyser.fftSize = 512;
                source.connect(this.analyser);

                // Time-domain RMS: measures actual waveform amplitude
                // Drops to near-zero during pauses, unlike FFT frequency averages
                const dataArray = new Uint8Array(this.analyser.fftSize);
                const tick = () => {
                    if (!this.analyser) return;
                    this.analyser.getByteTimeDomainData(dataArray);
                    let sumSq = 0;
                    for (let i = 0; i < dataArray.length; i++) {
                        const v = (dataArray[i] - 128) / 128;
                        sumSq += v * v;
                    }
                    const rms = Math.sqrt(sumSq / dataArray.length);
                    if (this.onAudioLevel) this.onAudioLevel(rms);
                    this.levelInterval = requestAnimationFrame(tick);
                };
                this.levelInterval = requestAnimationFrame(tick);
            }

            stopRecording() {
                return new Promise((resolve) => {
                    if (!this.mediaRecorder || this.mediaRecorder.state === 'inactive') {
                        resolve(null);
                        return;
                    }
                    this.mediaRecorder.onstop = () => {
                        const mimeType = this.mediaRecorder.mimeType || 'audio/webm';
                        const blob = new Blob(this.chunks, { type: mimeType });
                        this.isRecording = false;
                        resolve(blob);
                    };
                    this.mediaRecorder.stop();
                    this.cleanupMonitor();
                });
            }

            cleanupMonitor() {
                if (this.levelInterval) {
                    cancelAnimationFrame(this.levelInterval);
                    this.levelInterval = null;
                }
                if (this.audioContext) {
                    this.audioContext.close().catch(() => {});
                    this.audioContext = null;
                }
                this.analyser = null;
            }

            cleanup() {
                this.cleanupMonitor();
                if (this.stream) {
                    this.stream.getTracks().forEach(t => t.stop());
                    this.stream = null;
                }
                this.mediaRecorder = null;
                this.chunks = [];
                this.isRecording = false;
            }

            async transcribeAudio(blob) {
                const ext = blob.type.includes('ogg') ? 'ogg' : 'webm';
                const formData = new FormData();
                formData.append('file', blob, 'recording.' + ext);

                // Don't use getAuthHeaders() here - it sets Content-Type: application/json
                // which overrides the browser's automatic multipart/form-data boundary
                const headers = {};
                if (mobileToken) {
                    headers['Authorization'] = 'Bearer ' + mobileToken;
                }
                const resp = await fetch(API_BASE + '/transcribe', {
                    method: 'POST',
                    headers,
                    body: formData
                });

                if (handleAuthFailure(resp)) throw new Error('Auth failed');
                if (!resp.ok) {
                    const err = await resp.json();
                    throw new Error(err.error || 'Transcription failed');
                }

                const data = await resp.json();
                return data.text;
            }
        }

        const voiceState = new Map();

        function updateVoiceBars(sessionId, level) {
            const btn = document.getElementById('voice-btn-' + sessionId);
            if (!btn) return;
            const bars = btn.querySelectorAll('.voice-bar');

            const now = performance.now();
            let s = voiceState.get(sessionId);
            if (!s) {
                // Start with a generous peak estimate so first speech doesn't blow out.
                // Floor calibrates from the first sample; peak starts high and adapts down.
                s = { floor: level, peak: 0.15, lastTime: now, smooth: 0 };
                voiceState.set(sessionId, s);
            }

            const elapsed = now - s.lastTime;
            s.lastTime = now;

            // Noise floor: rises slowly to track ambient, drops instantly
            if (level < s.floor) {
                s.floor = level;
            } else {
                s.floor += (level - s.floor) * 0.01;
            }

            // Gate: subtract floor with 1.5x margin, clamp to 0
            const gated = Math.max(0, level - s.floor * 1.5);

            // Peak of gated signal: rises instantly, decays fast (halve per 200ms)
            s.peak = Math.max(s.peak * Math.pow(0.5, elapsed / 200), gated);

            // Normalize gated signal against peak
            const normalized = s.peak > 0.001 ? Math.min(1, gated / s.peak) : 0;

            // Time-based smoothing so it feels the same at any frame rate
            // Attack: reach target in ~50ms, Release: reach zero in ~400ms
            const attackRate = 1 - Math.pow(0.01, elapsed / 50);
            const releaseRate = 1 - Math.pow(0.01, elapsed / 400);
            const smoothFactor = normalized > s.smooth ? attackRate : releaseRate;
            s.smooth += (normalized - s.smooth) * smoothFactor;

            const shaped = Math.sqrt(s.smooth);

            const multipliers = [0.5, 0.8, 1.0, 0.85, 0.55];
            bars.forEach((bar, i) => {
                const height = Math.max(3, Math.min(24, shaped * multipliers[i] * 24));
                bar.style.height = height + 'px';
            });
        }

        function setVoiceState(sessionId, state) {
            const btn = document.getElementById('voice-btn-' + sessionId);
            if (!btn) return;
            btn.classList.remove('recording', 'transcribing');

            if (state === 'idle') {
                btn.innerHTML = '${iconMicrophone.replace(/'/g, "\\'")}';
            } else if (state === 'recording') {
                btn.classList.add('recording');
                btn.innerHTML = '<div class="voice-bars">' +
                    '<span class="voice-bar"></span>'.repeat(5) +
                    '</div>';
            } else if (state === 'transcribing') {
                btn.classList.add('transcribing');
                btn.innerHTML = '${iconMicrophone.replace(/'/g, "\\'")}';
            }
        }

        async function toggleVoiceRecording(sessionId) {
            if (!VoiceRecorder.isSupported()) {
                showVoiceError(sessionId, 'Not supported');
                return;
            }

            let recorder = voiceRecorders.get(sessionId);

            if (recorder && recorder.isRecording) {
                // Stop recording
                setVoiceState(sessionId, 'transcribing');
                try {
                    const blob = await recorder.stopRecording();
                    if (!blob || blob.size === 0) {
                        setVoiceState(sessionId, 'idle');
                        return;
                    }
                    const text = await recorder.transcribeAudio(blob);
                    if (text && text.trim()) {
                        const input = document.getElementById('input-' + sessionId);
                        if (input) {
                            input.value = text.trim();
                            sendAnswer(sessionId);
                        }
                    }
                } catch (err) {
                    console.error('Voice transcription error:', err);
                    showVoiceError(sessionId, err.message || 'Failed');
                } finally {
                    recorder.cleanup();
                    setVoiceState(sessionId, 'idle');
                }
                return;
            }

            // Start recording
            if (!recorder) {
                recorder = new VoiceRecorder();
                voiceRecorders.set(sessionId, recorder);
            }

            try {
                recorder.onAudioLevel = (level) => updateVoiceBars(sessionId, level);
                await recorder.startRecording();
                setVoiceState(sessionId, 'recording');
            } catch (err) {
                console.error('Voice recording error:', err);
                showVoiceError(sessionId, 'Mic denied');
                recorder.cleanup();
            }
        }

        function showVoiceError(sessionId, message) {
            const btn = document.getElementById('voice-btn-' + sessionId);
            if (!btn) return;
            btn.classList.add('voice-error');
            btn.title = message;
            setTimeout(() => {
                btn.classList.remove('voice-error');
                btn.title = 'Voice input';
            }, 2000);
        }
`;
