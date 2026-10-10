package com.crabigator.app

import android.content.Context
import android.media.MediaRecorder
import java.io.File

/** A short, foreground-only recording stored in the app cache until transcription or cancellation. */
internal class VoiceRecorder(private val context: Context) {
    private var recorder: MediaRecorder? = null
    private var file: File? = null

    @Suppress("DEPRECATION")
    fun start() {
        cancel()
        val output = File.createTempFile("voice-", ".m4a", context.cacheDir)
        file = output
        try {
            val recording = if (android.os.Build.VERSION.SDK_INT >= 31) MediaRecorder(context) else MediaRecorder()
            recorder = recording
            recording.setAudioSource(MediaRecorder.AudioSource.MIC)
            recording.setOutputFormat(MediaRecorder.OutputFormat.MPEG_4)
            recording.setAudioEncoder(MediaRecorder.AudioEncoder.AAC)
            recording.setAudioSamplingRate(44100)
            recording.setAudioEncodingBitRate(96000)
            recording.setOutputFile(output.absolutePath)
            recording.prepare()
            recording.start()
        } catch (e: Exception) { cancel(); throw e }
    }

    fun amplitude(): Float = (runCatching { recorder?.maxAmplitude ?: 0 }.getOrDefault(0) / 32767f).coerceIn(0f, 1f)

    fun finish(): File {
        val recording = checkNotNull(recorder)
        try { recording.stop() }
        catch (e: Exception) { file?.delete(); file = null; throw e }
        finally { recorder = null; runCatching { recording.release() } }
        return checkNotNull(file).also { file = null }
    }

    fun cancel() {
        val recording = recorder
        recorder = null
        recording?.let { runCatching { it.stop() }; runCatching { it.release() } }
        file?.delete()
        file = null
    }
}
