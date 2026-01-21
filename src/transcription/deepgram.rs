use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

use deepgram::common::options::{Encoding, Language, Model, Options};
use deepgram::common::stream_response::StreamResponse;
use deepgram::listen::websocket::WebsocketHandle;

use super::backend::{TranscriptionBackend, TranscriptionError};

/// Audio sample rate in Hz (16kHz mono)
const SAMPLE_RATE: u32 = 16_000;

/// Bytes per audio sample (i16 = 2 bytes)
const BYTES_PER_SAMPLE: usize = 2;

/// Chunk duration for streaming in milliseconds (250ms for low latency)
const CHUNK_DURATION_MS: usize = 250;

/// Bytes per streaming chunk: 16kHz * 2 bytes * 250ms = 8000 bytes
const CHUNK_SIZE_BYTES: usize = SAMPLE_RATE as usize * BYTES_PER_SAMPLE * CHUNK_DURATION_MS / 1000;

/// Buffer capacity for accumulating audio (500ms = 2 chunks)
const BUFFER_CAPACITY_BYTES: usize = CHUNK_SIZE_BYTES * 2;

/// Active streaming session state
struct StreamingSession {
    /// Channel to send audio chunks to the streaming task
    audio_tx: mpsc::UnboundedSender<Vec<u8>>,
    /// Channel to receive final transcript
    result_rx: tokio::sync::oneshot::Receiver<Result<String, TranscriptionError>>,
}

/// Deepgram cloud transcription backend using WebSocket streaming
pub struct DeepgramBackend {
    /// Deepgram client (shared across requests)
    client: Arc<deepgram::Deepgram>,
    /// Model name (e.g., "whisper-large", "nova-3")
    model: String,
    /// Language code (None = auto-detect)
    language: Option<String>,
    /// Enable smart formatting (punctuation, capitalization)
    smart_format: bool,
    /// Tokio runtime for async operations
    runtime: Arc<Runtime>,
    /// Active streaming session (if any)
    streaming_session: Mutex<Option<StreamingSession>>,
}

impl DeepgramBackend {
    /// Creates new Deepgram backend
    ///
    /// # Errors
    /// Returns error if client creation fails
    pub fn new(
        api_key: &str,
        model: String,
        language: Option<String>,
        smart_format: bool,
        runtime: Arc<Runtime>,
    ) -> Result<Self, TranscriptionError> {
        let client = deepgram::Deepgram::new(api_key)
            .map_err(|e| TranscriptionError::DeepgramApi(e.to_string()))?;

        Ok(Self {
            client: Arc::new(client),
            model,
            language,
            smart_format,
            runtime,
            streaming_session: Mutex::new(None),
        })
    }

    /// Converts f32 samples to raw PCM i16 little-endian bytes
    fn convert_to_pcm_i16(audio_data: &[f32]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(audio_data.len() * 2);
        for &sample in audio_data {
            #[allow(clippy::cast_possible_truncation)]
            let sample_i16 = (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16;
            bytes.extend_from_slice(&sample_i16.to_le_bytes());
        }
        bytes
    }

    /// Build options for streaming
    fn build_options(&self) -> Options {
        let options_builder = Options::builder()
            .model(Model::from(self.model.clone()))
            .smart_format(self.smart_format);
        if let Some(ref lang) = self.language {
            options_builder
                .language(Language::from(lang.clone()))
                .build()
        } else {
            options_builder.build()
        }
    }

    /// Run the streaming task that handles websocket I/O
    async fn run_streaming_task(
        client: Arc<deepgram::Deepgram>,
        options: Options,
        mut audio_rx: mpsc::UnboundedReceiver<Vec<u8>>,
        result_tx: tokio::sync::oneshot::Sender<Result<String, TranscriptionError>>,
    ) {
        let result = Self::run_streaming_inner(client, options, &mut audio_rx).await;
        let _ = result_tx.send(result);
    }

    #[allow(clippy::too_many_lines)]
    async fn run_streaming_inner(
        client: Arc<deepgram::Deepgram>,
        options: Options,
        audio_rx: &mut mpsc::UnboundedReceiver<Vec<u8>>,
    ) -> Result<String, TranscriptionError> {
        tracing::debug!("streaming task: opening websocket...");

        // Open websocket with timeout
        let connect_result = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            client
                .transcription()
                .stream_request_with_options(options)
                .encoding(Encoding::Linear16)
                .sample_rate(SAMPLE_RATE)
                .channels(1)
                .no_delay(true)
                .handle(),
        )
        .await;

        let mut handle: WebsocketHandle = connect_result
            .map_err(|_| TranscriptionError::DeepgramApi("websocket connect timeout".into()))?
            .map_err(|e| TranscriptionError::DeepgramApi(format!("websocket connect: {e}")))?;

        tracing::debug!("streaming task: websocket connected");

        // Buffer for accumulating audio before sending
        let mut buffer: Vec<u8> = Vec::with_capacity(BUFFER_CAPACITY_BYTES);
        let mut chunks_sent = 0;
        let mut total_bytes: usize = 0;

        // Process audio chunks until channel closes
        while let Some(pcm_bytes) = audio_rx.recv().await {
            if pcm_bytes.is_empty() {
                tracing::debug!("streaming task: received end signal");
                break;
            }

            total_bytes += pcm_bytes.len();
            buffer.extend(pcm_bytes);

            // Send when buffer reaches chunk threshold
            if buffer.len() >= CHUNK_SIZE_BYTES {
                handle
                    .send_data(std::mem::take(&mut buffer))
                    .await
                    .map_err(|e| TranscriptionError::DeepgramApi(format!("send data: {e}")))?;
                chunks_sent += 1;
            }
        }

        // Send remaining buffered audio
        if !buffer.is_empty() {
            tracing::debug!(
                remaining_bytes = buffer.len(),
                "streaming task: sending final buffer"
            );
            handle
                .send_data(buffer)
                .await
                .map_err(|e| TranscriptionError::DeepgramApi(format!("send final: {e}")))?;
            chunks_sent += 1;
        }

        tracing::debug!(
            chunks_sent,
            total_bytes,
            "streaming task: closing stream..."
        );

        // Close stream to signal end of audio
        handle
            .close_stream()
            .await
            .map_err(|e| TranscriptionError::DeepgramApi(format!("close_stream: {e}")))?;

        // Collect final transcripts
        let mut transcript_parts: Vec<String> = Vec::new();
        let receive_timeout = std::time::Duration::from_secs(30);
        let receive_start = std::time::Instant::now();

        loop {
            let remaining = receive_timeout.saturating_sub(receive_start.elapsed());
            if remaining.is_zero() {
                tracing::warn!("streaming task: receive timeout");
                break;
            }

            match tokio::time::timeout(remaining, handle.receive()).await {
                Ok(Some(response)) => match response {
                    Ok(StreamResponse::TranscriptResponse {
                        channel, is_final, ..
                    }) => {
                        if is_final {
                            if let Some(alt) = channel.alternatives.first() {
                                if !alt.transcript.is_empty() {
                                    tracing::debug!(
                                        transcript = %alt.transcript,
                                        "streaming task: final transcript part"
                                    );
                                    transcript_parts.push(alt.transcript.clone());
                                }
                            }
                        }
                    }
                    Ok(StreamResponse::TerminalResponse { .. }) => {
                        tracing::debug!("streaming task: terminal response");
                        break;
                    }
                    Ok(_) => {}
                    Err(e) => {
                        tracing::warn!(error = %e, "streaming task: receive error");
                    }
                },
                Ok(None) => {
                    tracing::debug!("streaming task: stream ended");
                    break;
                }
                Err(_) => {
                    tracing::warn!("streaming task: receive timeout");
                    break;
                }
            }
        }

        let transcript = transcript_parts.join(" ");
        tracing::debug!(len = transcript.len(), "streaming task: completed");
        Ok(transcript)
    }
}

impl TranscriptionBackend for DeepgramBackend {
    /// Transcribe audio using Deepgram WebSocket streaming API.
    ///
    /// **Note:** This method blocks the calling thread while waiting for the
    /// transcript result. Use from a background thread or blocking task pool.
    fn transcribe(&self, audio_data: &[f32]) -> Result<String, TranscriptionError> {
        // Batch transcription (fallback if streaming not used)
        let _span =
            tracing::debug_span!("deepgram_transcription", samples = audio_data.len()).entered();
        tracing::info!(
            model = %self.model,
            language = ?self.language,
            smart_format = self.smart_format,
            samples = audio_data.len(),
            "starting deepgram batch transcription"
        );

        let pcm_bytes = Self::convert_to_pcm_i16(audio_data);
        let options = self.build_options();
        let client = Arc::clone(&self.client);

        let (audio_tx, audio_rx) = mpsc::unbounded_channel();
        let (result_tx, result_rx) = tokio::sync::oneshot::channel();

        // Spawn streaming task
        self.runtime.spawn(Self::run_streaming_task(
            client, options, audio_rx, result_tx,
        ));

        // Send all audio at once
        audio_tx
            .send(pcm_bytes)
            .map_err(|_| TranscriptionError::DeepgramApi("failed to send audio".into()))?;
        // Signal end
        audio_tx
            .send(Vec::new())
            .map_err(|_| TranscriptionError::DeepgramApi("failed to send end signal".into()))?;

        // Wait for result
        self.runtime.block_on(async {
            result_rx
                .await
                .map_err(|_| TranscriptionError::DeepgramApi("streaming task dropped".into()))?
        })
    }

    fn backend_name(&self) -> &'static str {
        "deepgram"
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn start_stream(&self) -> Result<(), TranscriptionError> {
        let mut session_guard = self
            .streaming_session
            .lock()
            .map_err(|_| TranscriptionError::DeepgramApi("session lock poisoned".into()))?;

        if session_guard.is_some() {
            return Err(TranscriptionError::DeepgramApi(
                "stream already active".into(),
            ));
        }

        tracing::info!(
            model = %self.model,
            language = ?self.language,
            "starting deepgram streaming session"
        );

        let options = self.build_options();
        let client = Arc::clone(&self.client);

        let (audio_tx, audio_rx) = mpsc::unbounded_channel();
        let (result_tx, result_rx) = tokio::sync::oneshot::channel();

        // Spawn streaming task in background
        self.runtime.spawn(Self::run_streaming_task(
            client, options, audio_rx, result_tx,
        ));

        *session_guard = Some(StreamingSession {
            audio_tx,
            result_rx,
        });
        drop(session_guard);

        tracing::debug!("streaming session started");
        Ok(())
    }

    fn send_audio_chunk(&self, audio_data: &[f32]) -> Result<(), TranscriptionError> {
        let audio_tx = {
            let mut session_guard = self
                .streaming_session
                .lock()
                .map_err(|_| TranscriptionError::DeepgramApi("session lock poisoned".into()))?;

            let session = session_guard
                .as_ref()
                .ok_or_else(|| TranscriptionError::DeepgramApi("no active stream".into()))?;

            // Check if streaming task has failed/panicked (channel closed)
            if session.audio_tx.is_closed() {
                tracing::warn!("streaming task failed, clearing stale session");
                session_guard.take();
                drop(session_guard);
                return Err(TranscriptionError::DeepgramApi(
                    "streaming task failed".into(),
                ));
            }

            let tx = session.audio_tx.clone();
            drop(session_guard);
            tx
        };

        let pcm_bytes = Self::convert_to_pcm_i16(audio_data);
        audio_tx
            .send(pcm_bytes)
            .map_err(|_| TranscriptionError::DeepgramApi("streaming task closed".into()))?;

        tracing::trace!(samples = audio_data.len(), "sent audio chunk");
        Ok(())
    }

    fn finish_stream(&self) -> Result<String, TranscriptionError> {
        let session = {
            let mut session_guard = self
                .streaming_session
                .lock()
                .map_err(|_| TranscriptionError::DeepgramApi("session lock poisoned".into()))?;

            session_guard
                .take()
                .ok_or_else(|| TranscriptionError::DeepgramApi("no active stream".into()))?
        };

        tracing::debug!("finishing streaming session...");

        // Signal end of audio
        let _ = session.audio_tx.send(Vec::new());

        // Wait for result
        let result = self.runtime.block_on(async {
            session
                .result_rx
                .await
                .map_err(|_| TranscriptionError::DeepgramApi("streaming task dropped".into()))?
        });

        tracing::info!(
            transcript_len = result.as_ref().map(String::len).unwrap_or(0),
            "streaming session completed"
        );

        result
    }
}

// SAFETY: DeepgramBackend is thread-safe because:
// 1. Deepgram client is wrapped in Arc and is documented as thread-safe
// 2. streaming_session is protected by Mutex
// 3. Runtime is Arc-wrapped and thread-safe
#[allow(unsafe_code)]
unsafe impl Send for DeepgramBackend {}
#[allow(unsafe_code)]
unsafe impl Sync for DeepgramBackend {}
