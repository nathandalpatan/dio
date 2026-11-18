use super::{model_store, Backend, OpenedOutput, Output};
use ort::{
    environment::Environment,
    session::{builder::GraphOptimizationLevel, Session},
    value::TensorRef,
};
use std::collections::HashMap;
use std::num::NonZero;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

pub const SAMPLE_RATE: u32 = 24_000;
const MAX_TOKENS: usize = 510;
const STYLE: usize = 256;
static ENVIRONMENT: OnceLock<Result<Environment, String>> = OnceLock::new();

include!("vocab.rs");

pub fn tokens(phonemes: &str) -> Vec<i64> {
    phonemes
        .chars()
        .filter_map(|c| {
            VOCAB
                .iter()
                .find(|(symbol, _)| *symbol == c)
                .map(|(_, id)| *id)
        })
        .collect()
}

/// Splits token ids at word boundaries so every model call fits Kokoro's context window.
pub fn split_tokens(ids: &[i64]) -> Vec<&[i64]> {
    const SPACE: i64 = 16;
    let mut pieces = Vec::new();
    let mut start = 0;
    while ids.len() - start > MAX_TOKENS {
        let window = &ids[start..start + MAX_TOKENS];
        let cut = window
            .iter()
            .rposition(|id| *id == SPACE)
            .filter(|position| *position > 0)
            .map_or(MAX_TOKENS, |position| position + 1);
        pieces.push(&ids[start..start + cut]);
        start += cut;
    }
    if start < ids.len() {
        pieces.push(&ids[start..]);
    }
    pieces
}

pub struct Kokoro {
    session: Session,
    voices: HashMap<String, Vec<f32>>,
}

impl Kokoro {
    /// Loads verified model files; callers check them with [`model_store::verify`] first.
    pub fn load(dir: &Path, runtime: &Path) -> Result<Self, String> {
        let environment = ENVIRONMENT
            .get_or_init(|| {
                ort::init_from(runtime)
                    .map_err(|e| format!("The speech runtime could not be loaded: {e}"))?
                    .with_name("dio")
                    .with_telemetry(false)
                    .build()
                    .map_err(|e| format!("The speech runtime could not start: {e}"))
            })
            .as_ref()
            .map_err(Clone::clone)?;
        let threads = std::thread::available_parallelism()
            .map_or(2, |count| count.get())
            .clamp(1, 4);
        let failed =
            |e: &dyn std::fmt::Display| format!("The voice model could not be loaded: {e}");
        let session = Session::builder(environment)
            .map_err(|e| failed(&e))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| failed(&e))?
            .with_intra_threads(threads)
            .map_err(|e| failed(&e))?
            .commit_from_file(dir.join(model_store::MODEL))
            .map_err(|e| failed(&e))?;
        let mut voices = HashMap::new();
        for voice in super::VOICES {
            let bytes = std::fs::read(dir.join(format!("{}.bin", voice.id)))
                .map_err(|e| format!("The {} voice could not be read: {e}", voice.name))?;
            voices.insert(
                voice.id.to_owned(),
                bytes
                    .chunks_exact(4)
                    .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    .collect(),
            );
        }
        Ok(Self { session, voices })
    }

    fn run(&mut self, ids: &[i64], voice: &[f32], speed: f32) -> Result<Vec<f32>, String> {
        let frames = voice.len() / STYLE;
        let row = ids.len().saturating_sub(1).min(frames.saturating_sub(1));
        let style = &voice[row * STYLE..(row + 1) * STYLE];
        let mut input = Vec::with_capacity(ids.len() + 2);
        input.push(0);
        input.extend_from_slice(ids);
        input.push(0);
        let speed = [speed];
        let outputs = self
            .session
            .run(ort::inputs![
                "input_ids" => TensorRef::from_array_view(([1usize, input.len()], input.as_slice())).map_err(|e| e.to_string())?,
                "style" => TensorRef::from_array_view(([1usize, STYLE], style)).map_err(|e| e.to_string())?,
                "speed" => TensorRef::from_array_view(([1usize], &speed[..])).map_err(|e| e.to_string())?,
            ])
            .map_err(|e| format!("Speech synthesis failed: {e}"))?;
        let (_, audio) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| format!("Speech synthesis failed: {e}"))?;
        Ok(audio.to_vec())
    }
}

impl Backend for Kokoro {
    fn synthesize(
        &mut self,
        text: &str,
        voice: &str,
        british: bool,
        speed: f32,
    ) -> Result<Vec<f32>, String> {
        let style = self
            .voices
            .get(voice)
            .ok_or_else(|| format!("The voice {voice} is not installed."))?
            .clone();
        let ids = tokens(&super::g2p::speech_phonemes(text, british));
        let mut audio = Vec::new();
        for piece in split_tokens(&ids) {
            audio.extend(self.run(piece, &style, speed)?);
        }
        Ok(audio)
    }
}

pub struct Speakers {
    player: rodio::Player,
    failed: Arc<AtomicBool>,
}

/// Opens the default output device. The returned guard owns the device stream and must stay
/// on the thread that opened it.
pub fn open_speakers() -> Result<OpenedOutput, String> {
    let failed = Arc::new(AtomicBool::new(false));
    let flag = failed.clone();
    let mut device = rodio::DeviceSinkBuilder::from_default_device()
        .map_err(|_| "No audio output device is available.".to_string())?
        .with_error_callback(move |_| flag.store(true, Ordering::Release))
        .open_stream()
        .map_err(|e| format!("The audio device could not be opened: {e}"))?;
    device.log_on_drop(false);
    let player = rodio::Player::connect_new(device.mixer());
    Ok((Arc::new(Speakers { player, failed }), Box::new(device)))
}

impl Output for Speakers {
    fn append(&self, samples: Vec<f32>) {
        self.player.append(rodio::buffer::SamplesBuffer::new(
            NonZero::new(1).unwrap(),
            NonZero::new(SAMPLE_RATE).unwrap(),
            samples,
        ));
    }
    fn queued(&self) -> usize {
        self.player.len()
    }
    fn finished(&self) -> bool {
        self.player.empty()
    }
    fn failed(&self) -> bool {
        self.failed.load(Ordering::Acquire)
    }
    fn pause(&self) {
        self.player.pause();
    }
    fn play(&self) {
        self.player.play();
    }
    fn stop(&self) {
        self.player.stop();
    }
    fn set_volume(&self, volume: f32) {
        self.player.set_volume(volume);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_kokoro_phonemes_and_drops_unknown_symbols() {
        assert_eq!(tokens("hˈi ❓."), vec![50, 156, 51, 16, 4]);
    }

    #[test]
    fn splits_long_input_at_word_boundaries() {
        let mut ids = Vec::new();
        for _ in 0..300 {
            ids.extend([53, 54, 16]);
        }
        let pieces = split_tokens(&ids);
        assert!(pieces.iter().all(|piece| piece.len() <= MAX_TOKENS));
        assert!(pieces[..pieces.len() - 1]
            .iter()
            .all(|piece| piece.last() == Some(&16)));
        assert_eq!(
            pieces.iter().map(|piece| piece.len()).sum::<usize>(),
            ids.len()
        );
        let unbroken = vec![53; 1200];
        assert_eq!(
            split_tokens(&unbroken)
                .iter()
                .map(|piece| piece.len())
                .collect::<Vec<_>>(),
            [510, 510, 180]
        );
    }

    /// Downloads the pinned model through DIO's installer (network required) into
    /// DIO_MODEL_TEST_DIR, or a temporary folder, and synthesizes on the CPU:
    /// `cargo test -- --ignored`.
    #[tokio::test]
    #[ignore]
    async fn installs_and_synthesizes_with_the_real_model_on_cpu() {
        let root = std::env::var("DIO_MODEL_TEST_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir().join("dio-model-test"));
        let runtime = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/onnxruntime.dll");
        let dir = model_store::directory(&root);
        model_store::install(
            &model_store::client().unwrap(),
            model_store::BASE_URL,
            &dir,
            &model_store::assets(),
            &AtomicBool::new(false),
            |_, _| {},
        )
        .await
        .unwrap();
        let started = std::time::Instant::now();
        model_store::verify(&dir, &model_store::assets()).unwrap();
        let verified = started.elapsed();
        let started = std::time::Instant::now();
        let mut kokoro = Kokoro::load(&dir, &runtime).unwrap();
        let loaded = started.elapsed();
        let started = std::time::Instant::now();
        let audio = kokoro
            .synthesize("serendipity", "af_heart", false, 1.0)
            .unwrap();
        eprintln!(
            "verify {verified:?}, load {loaded:?}, synthesis {:?}",
            started.elapsed()
        );
        assert!(audio.len() > SAMPLE_RATE as usize / 2);
        assert!(audio.iter().any(|sample| sample.abs() > 0.01));
        let british = kokoro.synthesize("Tomato.", "bf_emma", true, 1.5).unwrap();
        assert!(!british.is_empty());
        let started = std::time::Instant::now();
        let long = kokoro
            .synthesize(
                &"The quick brown fox jumps over the lazy dog. ".repeat(12),
                "am_michael",
                false,
                1.0,
            )
            .unwrap();
        eprintln!(
            "{:.1}s of audio in {:?}",
            long.len() as f32 / SAMPLE_RATE as f32,
            started.elapsed()
        );
    }
}
