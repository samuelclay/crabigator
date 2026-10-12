import AVFoundation
import SwiftUI

@MainActor final class VoiceRecorder: NSObject, ObservableObject, AVAudioRecorderDelegate {
    @Published var recording = false
    @Published var processing = false
    @Published var levels = Array(repeating: CGFloat(0), count: 32)
    @Published var seconds = 0
    @Published var error: String?
    private var recorder: AVAudioRecorder?, timer: Task<Void, Never>?, transcription: Task<Void, Never>?
    private var file: URL?, generation = 0
    var timedOut: (() -> Void)?
    func start() async {
        generation += 1; let epoch = generation
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
            timer = Task {
                let start = Date()
                while !Task.isCancelled && recording {
                    recorder?.updateMeters(); let power = recorder?.averagePower(forChannel: 0) ?? -80
                    levels = Array(levels.dropFirst()) + [min(1, CGFloat(pow(10, power / 40)))]
                    seconds = Int(Date().timeIntervalSince(start))
                    if seconds >= 120 { timedOut?(); break }
                    try? await Task.sleep(for: .milliseconds(50))
                }
            }
        } catch { self.error = error.localizedDescription; cancel() }
    }
    func finish(_ completion: @escaping (String) -> Void) {
        guard recording, let url = file else { return }
        recorder?.stop(); recorder = nil; recording = false; processing = true; timer?.cancel()
        try? AVAudioSession.sharedInstance().setActive(false)
        let epoch = generation
        transcription = Task {
            defer { try? FileManager.default.removeItem(at: url); if epoch == generation { file = nil; processing = false } }
            do {
                let text = try await API.shared.transcribe(url)
                guard !Task.isCancelled, epoch == generation else { return }
                if text.isEmpty { error = "No speech detected." } else { completion(text) }
            } catch { if !Task.isCancelled && epoch == generation { self.error = error.localizedDescription } }
        }
    }
    func cancel() {
        generation += 1; timer?.cancel(); transcription?.cancel(); recorder?.stop(); recorder = nil
        if let file { try? FileManager.default.removeItem(at: file) }; file = nil; recording = false; processing = false
        try? AVAudioSession.sharedInstance().setActive(false)
    }
}
