import AVFoundation
import Foundation
import Speech

let outputPath = CommandLine.arguments.dropFirst().first ?? ""
let outputURL = URL(fileURLWithPath: outputPath)
let silenceSeconds = 1.5
let maximumSeconds = 8.0
let recognizer = SFSpeechRecognizer(locale: Locale(identifier: "zh-CN"))!
let request = SFSpeechAudioBufferRecognitionRequest()
let engine = AVAudioEngine()
let lock = NSLock()
var lastTranscript = ""
var heardSpeech = false
var lastSpeechAt = Date()
var startedAt = Date()
var ended = false

func emit(_ kind: String, _ value: String = "") {
    let line = "\(kind)\t\(value.replacingOccurrences(of: "\n", with: " "))\n"
    lock.lock()
    if let handle = try? FileHandle(forWritingTo: outputURL) {
        handle.seekToEndOfFile()
        handle.write(line.data(using: .utf8)!)
        try? handle.close()
    } else {
        try? line.write(to: outputURL, atomically: true, encoding: .utf8)
    }
    lock.unlock()
}

func stop(_ final: String? = nil) {
    lock.lock()
    if ended { lock.unlock(); return }
    ended = true
    lock.unlock()
    engine.stop()
    engine.inputNode.removeTap(onBus: 0)
    request.endAudio()
    emit("final", final ?? lastTranscript)
    exit(0)
}

SFSpeechRecognizer.requestAuthorization { status in
    guard status == .authorized else {
        emit("unavailable", "Speech recognition permission was not granted")
        exit(1)
    }
    guard recognizer.supportsOnDeviceRecognition else {
        emit("unavailable", "On-device speech recognition is unavailable")
        exit(1)
    }
    request.requiresOnDeviceRecognition = true
    request.shouldReportPartialResults = true
    let input = engine.inputNode
    let format = input.outputFormat(forBus: 0)
    guard format.sampleRate > 0 && format.channelCount > 0 else {
        emit("unavailable", "No microphone input is available")
        exit(1)
    }
    input.installTap(onBus: 0, bufferSize: 1024, format: format) { buffer, _ in
        request.append(buffer)
    }
    do {
        engine.prepare()
        try engine.start()
        startedAt = Date()
        lastSpeechAt = startedAt
    } catch {
        emit("unavailable", "Could not start microphone: \(error.localizedDescription)")
        exit(1)
    }
    recognizer.recognitionTask(with: request) { result, error in
        if let result {
            let text = result.bestTranscription.formattedString
            if !text.isEmpty {
                lastTranscript = text
                heardSpeech = true
                lastSpeechAt = Date()
                emit(result.isFinal ? "final" : "partial", text)
            }
            if result.isFinal { stop(text) }
        } else if error != nil {
            stop()
        }
    }
}

Timer.scheduledTimer(withTimeInterval: 0.1, repeats: true) { _ in
    let now = Date()
    if now.timeIntervalSince(startedAt) >= maximumSeconds ||
       (heardSpeech && now.timeIntervalSince(lastSpeechAt) >= silenceSeconds) {
        stop()
    }
}
RunLoop.main.run()
