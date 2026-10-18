import SwiftUI

struct SessionComposer: View {
    @ObservedObject var stream: SessionStream
    @ObservedObject var model: SessionModel
    var sessionID: String, active: Bool
    @State private var draft = ""
    @State private var keys = false
    @StateObject private var voice = VoiceRecorder()
    @State private var focused = false
    @Environment(\.scenePhase) private var phase
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    private var voiceVisible: Bool { voice.recording || voice.processing }
    private var composerAnimation: Animation { reduceMotion ? .easeOut(duration: 0.15) : .spring(response: 0.4, dampingFraction: 0.86) }
    private var controlTransition: AnyTransition { reduceMotion ? .opacity : .opacity.combined(with: .scale(scale: 0.8)).combined(with: .move(edge: .trailing)) }
    private var contentTransition: AnyTransition { reduceMotion ? .opacity : .opacity.combined(with: .offset(y: 8)) }
    var enabled: Bool { active && stream.connected && !stream.sending && !voice.processing }
    var blank: Bool { draft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty }
    var body: some View {
        VStack(spacing: 0) {
            Divider()
            if let error = voice.error {
                HStack { Text(error).font(.caption).foregroundStyle(Crab.red); Spacer(); IconButton(icon: "xmark", label: "Dismiss recording error") { voice.error = nil } }
                    .padding(.leading, 12).transition(reduceMotion ? .opacity : .opacity.combined(with: .move(edge: .top)))
            }
            HStack(alignment: .bottom, spacing: 4) {
                // Keep the leading button in place as the microphone becomes Cancel.
                IconButton(icon: voiceVisible ? "xmark" : "mic", label: voiceVisible ? "Cancel recording" : "Record voice", selected: voiceVisible, enabled: voiceVisible || enabled) {
                    if voiceVisible { voice.cancel() }
                    else { focused = false; keys = false; Task { await voice.start() } }
                }.contentTransition(reduceMotion ? .opacity : .symbolEffect(.replace))
                if !voiceVisible {
                    IconButton(icon: "keyboard", label: "Terminal keys", selected: keys) { focused = false; keys.toggle() }
                        .popover(isPresented: $keys, attachmentAnchor: .rect(.bounds), arrowEdge: .bottom) { TerminalKeyboard(codex: stream.selected?.platform == "codex", enabled: enabled) { bytes in stream.send(TerminalAction(route: "key-sequence", body: ["steps": [["type": "text", "text": bytes]]])) }.nativePopover() }
                        .transition(controlTransition)
                }
                ZStack {
                    if voiceVisible {
                        VStack(spacing: 4) {
                            ZStack {
                                if voice.processing { ProgressView().tint(Crab.orange).transition(.opacity) }
                                else { MicrophoneWaveform(levels: voice.levels).transition(.opacity) }
                            }.frame(height: 40)
                            Text(voice.processing ? "Transcribing…" : "\(voice.seconds / 60):\(String(format: "%02d", voice.seconds % 60)) / 2:00")
                                .font(.system(size: 11, design: .monospaced)).foregroundStyle(Crab.muted).contentTransition(.numericText())
                        }.padding(.horizontal, 10).padding(.vertical, 8).transition(contentTransition)
                    } else {
                        MessageField(placeholder: "Message", text: $draft, focused: $focused) { if enabled && !blank { send() } }
                            .padding(.horizontal, 12).padding(.vertical, 11).frame(minHeight: 44).transition(contentTransition)
                    }
                }.frame(maxWidth: .infinity)
                    .background(RoundedRectangle(cornerRadius: 14).fill(Crab.orange.opacity(voiceVisible ? 0.07 : 0)))
                    .overlay(RoundedRectangle(cornerRadius: 14).stroke(voiceVisible ? Crab.orange.opacity(0.4) : Color.white.opacity(0.2)))
                if voiceVisible {
                    IconButton(icon: "pencil", label: "Edit recording", enabled: voice.recording) { finish(send: false) }.transition(controlTransition)
                }
                if voiceVisible || !blank {
                    IconButton(icon: "arrow.up", label: voiceVisible ? "Send recording" : "Send", selected: true, enabled: voiceVisible ? voice.recording && stream.connected : enabled) {
                        if voiceVisible { finish(send: true) } else { send() }
                    }.transition(controlTransition)
                }
            }.padding(8)
        }.background(Crab.ink)
            .animation(composerAnimation, value: voiceVisible)
            .animation(composerAnimation, value: voice.processing)
            .animation(composerAnimation, value: voice.error)
            .animation(composerAnimation, value: blank)
            .onAppear { draft = model.drafts[sessionID] ?? ""; voice.timedOut = { finish(send: false) } }
            .onChange(of: draft) { _, value in model.drafts[sessionID] = value }
            .onChange(of: active) { _, value in if !value { voice.cancel(); focused = false; keys = false } }
            .onChange(of: model.navigation.conductor) { _, value in if value { voice.cancel(); focused = false; keys = false } }
            .onChange(of: phase) { _, value in if value != .active { voice.cancel() } }
            .onDisappear { voice.cancel() }
    }
    func finish(send immediately: Bool) {
        voice.finish { text in
            guard active, model.selectionID == sessionID else { return }
            draft = [draft.trimmingCharacters(in: .whitespacesAndNewlines), text].filter { !$0.isEmpty }.joined(separator: "\n")
            if immediately { send() } else { focused = true }
        }
    }
    func send() {
        guard active, model.selectionID == sessionID else { return }; let text = draft
        stream.send(TerminalAction(route: "answer", body: ["text": text])) { if draft == text { draft = "" } }
    }
}
/// Real microphone energy, with enough height and contrast to read at a glance.
private struct MicrophoneWaveform: View {
    let levels: [CGFloat]
    var body: some View {
        Canvas { context, size in
            let step = size.width / CGFloat(max(1, levels.count))
            for (i, level) in levels.enumerated() {
                let height = max(3, level * size.height)
                let rect = CGRect(x: (CGFloat(i) + 0.18) * step, y: (size.height - height) / 2, width: step * 0.64, height: height)
                context.fill(Path(roundedRect: rect, cornerRadius: 2), with: .color(Crab.orange))
            }
        }.accessibilityLabel("Microphone waveform")
    }
}

struct TerminalKeyboard: View {
    var codex: Bool, enabled: Bool
    var send: (String) -> Void
    @State private var shift = false
    @State private var control = false
    @State private var alt = false
    @State private var letters = false
    var body: some View {
        ScrollView {
            VStack(spacing: 10) {
                Text("Terminal keys").font(.headline).padding(.top, 16)
                HStack { modifier("Shift", value: $shift); modifier("Ctrl", value: $control); modifier("Alt", value: $alt) }
                HStack { shortcut("Mode", bytes: "\u{1b}[Z"); shortcut("Interrupt", bytes: "\u{3}"); shortcut(codex ? "Questions" : "Clear screen", hint: codex ? "Shift + ←" : nil, bytes: codex ? terminalKey("←", shift: true) : "\u{c}") }
                HStack { shortcut("Line start", bytes: "\u{1}"); shortcut("Line end", bytes: "\u{5}"); shortcut("Clear line", bytes: "\u{15}") }
                if !letters {
                keyRow(["Esc", "Tab", "Enter", "⌫"])
                keyRow(["Home", "↑", "End", "PgUp"])
                keyRow(["←", "↓", "→", "PgDn"])
                keyRow(["Insert", "Delete", "Space"])
                } else {
                    ForEach(["qwertyuiop", "asdfghjkl", "zxcvbnm"], id: \.self) { row in keyRow(row.map(String.init)).padding(.horizontal, CGFloat(10 - row.count) * 13) }
                    keyRow(["Space", "⌫", "Enter"])
                }
                HStack {
                    Button("Reset modifiers") { shift = false; control = false; alt = false }.font(.caption).frame(minHeight: 44)
                    Spacer()
                    Button(letters ? "Navigation" : "QWERTY") { letters.toggle() }.font(.caption).frame(minHeight: 44)
                }
            }.padding(12)
        }.background(Crab.ink)
    }
    func modifier(_ label: String, value: Binding<Bool>) -> some View { Button { value.wrappedValue.toggle() } label: { Text(label).frame(maxWidth: .infinity, minHeight: 44).background(value.wrappedValue ? Crab.orange.opacity(0.3) : Crab.panel, in: RoundedRectangle(cornerRadius: 6)) }.buttonStyle(.plain).accessibilityValue(value.wrappedValue ? "On" : "Off") }
    func shortcut(_ label: String, hint: String? = nil, bytes: String) -> some View {
        Button { send(bytes) } label: {
            VStack(spacing: 3) { Text(label); if let hint { Text(hint).font(.system(size: 10)).foregroundStyle(Crab.muted) } }
                .frame(maxWidth: .infinity, minHeight: 44)
        }.font(.system(size: 12)).buttonStyle(.bordered).disabled(!enabled).accessibilityLabel(label).accessibilityIdentifier(label)
    }
    func keyRow(_ keys: [String]) -> some View {
        HStack(spacing: 3) { ForEach(keys, id: \.self) { key in Button { send(terminalKey(key, shift: shift, control: control, alt: alt)) } label: { Text(shift && key.count == 1 ? key.uppercased() : key).font(.system(size: key.count > 1 ? 11 : 14, design: .monospaced)).frame(maxWidth: .infinity, minHeight: 40).background(Crab.panel, in: RoundedRectangle(cornerRadius: 5)) }.buttonStyle(.plain).disabled(!enabled) } }
    }
}
