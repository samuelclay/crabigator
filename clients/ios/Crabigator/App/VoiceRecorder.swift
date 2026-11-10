import AVFoundation
import SwiftUI

@MainActor final class VoiceRecorder: NSObject, ObservableObject, AVAudioRecorderDelegate {
    @Published var recording = false
    @Published var processing = false
    @Published var levels = Array(repeating: CGFloat(0), count: 32)
    @Published var seconds = 0
    @Published var error: String?
    private var recorder: AVAudioRecorder?, timer: Task<Void, Never>?, transcription: Task<Void, Never>?
    private var file: URL?, generation = 0, started = Date()
    /// A recording whose transcription failed. It is kept so the user can try again.
    @Published private(set) var failed: Upload?
    struct Upload { var url: URL, seconds: Double, sessionID: String, title: String }
    /// Set when the user has left the composer: the transcript still lands in the draft, but nothing is sent or focused.
    private(set) var quiet = false
    var timedOut: (() -> Void)?
    // A small noise floor keeps silence calm; speech gets a larger visual range.
    static func displayLevel(power: Float) -> CGFloat {
        min(1, max(0, CGFloat(pow(10, Double(power) / 40) - 0.018) * 2.4))
    }
    #if DEBUG
    private var preview: Bool { ProcessInfo.processInfo.environment["CRABIGATOR_TEST_VOICE"] == "1" }
    #endif
    func start() async {
        generation += 1; quiet = false; let epoch = generation
        #if DEBUG
        if preview {
            recording = true; error = nil; seconds = 0; levels = Array(repeating: 0, count: 32)
            timer = Task {
                let powers: [Float] = [-65, -58, -40, -32, -26, -20, -24, -29, -35, -48, -62, -55, -36, -23, -18, -30]
                var sample = 0
                while !Task.isCancelled && recording {
                    levels = Array(levels.dropFirst()) + [Self.displayLevel(power: powers[sample % powers.count])]
                    sample += 1; seconds = sample / 20
                    try? await Task.sleep(for: .milliseconds(50))
                }
            }
            return
        }
        #endif
        let granted = await AVAudioApplication.requestRecordPermission()
        guard epoch == generation else { return }
        guard granted else { error = "Microphone access is off."; return }
        do {
            let session = AVAudioSession.sharedInstance()
            try session.setCategory(.record, mode: .measurement); try session.setActive(true)
            let url = FileManager.default.temporaryDirectory.appendingPathComponent("crabigator-\(UUID().uuidString).m4a"); file = url
            recorder = try AVAudioRecorder(url: url, settings: [AVFormatIDKey: kAudioFormatMPEG4AAC, AVSampleRateKey: 44100, AVNumberOfChannelsKey: 1, AVEncoderBitRateKey: 64000])
            recorder?.isMeteringEnabled = true; recorder?.delegate = self
            guard recorder?.record() == true else { throw APIError(status: 0, message: "Could not start the microphone.") }
            recording = true; error = nil; seconds = 0; levels = Array(repeating: 0, count: 32)
            discardFailed(); started = Date()
            // Nobody touches the screen while dictating, and locking would end the recording.
            UIApplication.shared.isIdleTimerDisabled = true
            timer = Task {
                let start = started
                while !Task.isCancelled && recording {
                    recorder?.updateMeters(); let power = recorder?.averagePower(forChannel: 0) ?? -80
                    levels = Array(levels.dropFirst()) + [Self.displayLevel(power: power)]
                    seconds = Int(Date().timeIntervalSince(start))
                    if seconds >= 120 { timedOut?(); break }
                    try? await Task.sleep(for: .milliseconds(50))
                }
            }
        } catch { self.error = error.localizedDescription; cancel() }
    }
    func finish(sessionID: String, title: String, _ completion: @escaping (String) -> Void) {
        #if DEBUG
        if preview && recording {
            recording = false; processing = true; timer?.cancel()
            let epoch = generation
            transcription = Task {
                try? await Task.sleep(for: .milliseconds(800))
                guard !Task.isCancelled, epoch == generation else { return }
                completion("Sample voice transcript"); processing = false
            }
            return
        }
        #endif
        guard recording, let url = file else { return }
        let seconds = Date().timeIntervalSince(started)
        recorder?.stop(); recorder = nil; recording = false; timer?.cancel(); file = nil
        UIApplication.shared.isIdleTimerDisabled = false
        try? AVAudioSession.sharedInstance().setActive(false)
        transcribe(Upload(url: url, seconds: seconds, sessionID: sessionID, title: title), completion)
    }
    func retry(_ completion: @escaping (String) -> Void) {
        guard let upload = failed, !processing else { return }
        failed = nil; transcribe(upload, completion)
    }
    private func transcribe(_ upload: Upload, _ completion: @escaping (String) -> Void) {
        processing = true; error = nil
        let epoch = generation
        // The upload often starts as the app goes to the background; ask for time to finish it.
        let background = UIApplication.shared.beginBackgroundTask(withName: "Transcribe recording")
        transcription = Task {
            defer { UIApplication.shared.endBackgroundTask(background) }
            do {
                let text = try await API.shared.transcribe(upload.url, sessionID: upload.sessionID, title: upload.title, seconds: upload.seconds)
                try? FileManager.default.removeItem(at: upload.url)
                // A cancelled transcript is not delivered, but the server has already saved it to the history.
                guard !Task.isCancelled, epoch == generation else { return }
                processing = false
                if text.isEmpty { error = "No speech detected." } else { completion(text) }
            } catch {
                guard !Task.isCancelled, epoch == generation else { try? FileManager.default.removeItem(at: upload.url); return }
                processing = false
                // Nobody is there to press Try again once the composer is gone.
                if quiet { try? FileManager.default.removeItem(at: upload.url) } else { failed = upload; self.error = error.localizedDescription }
            }
        }
    }
    func dismissError() { error = nil; discardFailed() }
    private func discardFailed() {
        if let failed { try? FileManager.default.removeItem(at: failed.url) }; failed = nil
    }
    /// The user moved away. Drop a live recording, but let a transcript that is on its way arrive.
    func leave() { if processing { quiet = true } else { cancel() } }
    func cancel() {
        generation += 1; timer?.cancel(); transcription?.cancel(); recorder?.stop(); recorder = nil
        if let file { try? FileManager.default.removeItem(at: file) }; file = nil; recording = false; processing = false
        discardFailed()
        UIApplication.shared.isIdleTimerDisabled = false
        try? AVAudioSession.sharedInstance().setActive(false)
    }
}
