use reqwest::{redirect, Client, Url};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Pinned onnx-community/Kokoro-82M-v1.0-ONNX commit.
pub const REVISION: &str = "1939ad2a8e416c0acfeecc08a694d14ef25f2231";
pub const BASE_URL: &str = "https://huggingface.co/onnx-community/Kokoro-82M-v1.0-ONNX/resolve/";
pub const MODEL: &str = "model_quantized.onnx";
const MODEL_SIZE: u64 = 92_361_116;
const MODEL_SHA256: &str = "fbae9257e1e05ffc727e951ef9b9c98418e6d79f1c9b6b13bd59f5c9028a1478";
pub const VOICE_SIZE: u64 = 522_240;

#[derive(Clone, Debug, PartialEq)]
pub struct Asset {
    pub remote: String,
    pub local: String,
    pub size: u64,
    pub sha256: &'static str,
}

pub fn assets() -> Vec<Asset> {
    std::iter::once(Asset {
        remote: format!("onnx/{MODEL}"),
        local: MODEL.into(),
        size: MODEL_SIZE,
        sha256: MODEL_SHA256,
    })
    .chain(super::VOICES.iter().map(|voice| Asset {
        remote: format!("voices/{}.bin", voice.id),
        local: format!("{}.bin", voice.id),
        size: VOICE_SIZE,
        sha256: voice.sha256,
    }))
    .collect()
}

pub fn directory(app_data: &Path) -> PathBuf {
    app_data.join("kokoro").join(&REVISION[..12])
}

pub fn installed(dir: &Path, assets: &[Asset]) -> bool {
    assets.iter().all(|asset| {
        fs::metadata(dir.join(&asset.local)).is_ok_and(|meta| meta.len() == asset.size)
    })
}

fn hash_file(path: &Path) -> std::io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; 1 << 16];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex(&hasher.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Confirms every installed file still matches its pinned checksum.
pub fn verify(dir: &Path, assets: &[Asset]) -> Result<(), String> {
    for asset in assets {
        let path = dir.join(&asset.local);
        if hash_file(&path).ok().as_deref() != Some(asset.sha256) {
            return Err(format!("{} is missing or damaged.", asset.local));
        }
    }
    Ok(())
}

pub fn remove(dir: &Path) -> Result<(), String> {
    match fs::remove_dir_all(dir) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            Err(format!("Could not remove the voice model: {error}"))
        }
        _ => Ok(()),
    }
}

/// Hugging Face serves LFS files through redirects to its own CDN hosts.
pub fn client() -> Result<Client, String> {
    Client::builder()
        .user_agent("DIO/0.1 (desktop dictionary)")
        .connect_timeout(Duration::from_secs(10))
        .read_timeout(Duration::from_secs(30))
        .redirect(redirect::Policy::custom(|attempt| {
            let url = attempt.url();
            let host = url.host_str().unwrap_or("");
            if attempt.previous().len() >= 5 {
                attempt.error("too many redirects")
            } else if url.scheme() == "https"
                && (host == "huggingface.co"
                    || host.ends_with(".huggingface.co")
                    || host.ends_with(".hf.co"))
            {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        .build()
        .map_err(|e| e.to_string())
}

/// Downloads missing or damaged files into `dir`, verifying each against its pinned SHA-256
/// before it replaces anything. Partial files are deleted on failure.
pub async fn install(
    client: &Client,
    base: &str,
    dir: &Path,
    assets: &[Asset],
    cancel: &AtomicBool,
    mut progress: impl FnMut(u64, u64),
) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| format!("Could not create the model folder: {e}"))?;
    let total: u64 = assets.iter().map(|asset| asset.size).sum();
    let mut done = 0;
    for asset in assets {
        let target = dir.join(&asset.local);
        if fs::metadata(&target).is_ok_and(|meta| meta.len() == asset.size)
            && hash_file(&target).ok().as_deref() == Some(asset.sha256)
        {
            done += asset.size;
            progress(done, total);
            continue;
        }
        let part = target.with_extension("part");
        let result = download(client, base, asset, &part, cancel, |bytes| {
            progress(done + bytes, total)
        })
        .await;
        if let Err(error) = result {
            let _ = fs::remove_file(&part);
            return Err(error);
        }
        fs::rename(&part, &target).map_err(|e| {
            let _ = fs::remove_file(&part);
            format!("Could not save {}: {e}", asset.local)
        })?;
        done += asset.size;
        progress(done, total);
    }
    Ok(())
}

async fn download(
    client: &Client,
    base: &str,
    asset: &Asset,
    part: &Path,
    cancel: &AtomicBool,
    mut progress: impl FnMut(u64),
) -> Result<(), String> {
    let url = Url::parse(&format!("{base}{REVISION}/{}", asset.remote))
        .map_err(|_| "Invalid model address.".to_string())?;
    let network = |e: reqwest::Error| {
        if e.is_timeout() {
            "The model download timed out. Try again.".to_string()
        } else {
            "Could not download the voice model. Check your connection and try again.".to_string()
        }
    };
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(network)?
        .error_for_status()
        .map_err(|e| {
            format!(
                "The model server returned {}.",
                e.status().unwrap_or_default()
            )
        })?;
    let mut file = fs::File::create(part).map_err(|e| format!("Could not write the model: {e}"))?;
    let mut hasher = Sha256::new();
    let mut written = 0;
    while let Some(chunk) = response.chunk().await.map_err(network)? {
        if cancel.load(Ordering::Acquire) {
            return Err("Download cancelled.".into());
        }
        written += chunk.len() as u64;
        if written > asset.size {
            return Err(format!("{} is larger than expected.", asset.local));
        }
        hasher.update(&chunk);
        file.write_all(&chunk)
            .map_err(|e| format!("Could not write the model: {e}"))?;
        progress(written);
    }
    file.sync_all()
        .map_err(|e| format!("Could not write the model: {e}"))?;
    if written != asset.size || hex(&hasher.finalize()) != asset.sha256 {
        return Err(format!("{} failed verification.", asset.local));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    fn asset(body: &[u8]) -> Asset {
        Asset {
            remote: "voices/test.bin".into(),
            local: "test.bin".into(),
            size: body.len() as u64,
            sha256: Box::leak(hex(&Sha256::digest(body)).into_boxed_str()),
        }
    }

    fn serve(bodies: Vec<&'static [u8]>) -> (String, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}/", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            for body in bodies {
                let (mut socket, _) = listener.accept().unwrap();
                let mut request = [0; 4096];
                let _ = socket.read(&mut request);
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                socket.write_all(head.as_bytes()).unwrap();
                socket.write_all(body).unwrap();
            }
        });
        (base, server)
    }

    fn temporary(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dio-model-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[tokio::test]
    async fn installs_verified_files_and_skips_valid_ones() {
        let dir = temporary("ok");
        let assets = vec![asset(b"voice data")];
        let (base, server) = serve(vec![b"voice data"]);
        let mut seen = vec![];
        install(
            &Client::new(),
            &base,
            &dir,
            &assets,
            &AtomicBool::new(false),
            |done, total| seen.push((done, total)),
        )
        .await
        .unwrap();
        server.join().unwrap();
        assert!(installed(&dir, &assets));
        assert!(verify(&dir, &assets).is_ok());
        assert_eq!(seen.last(), Some(&(10, 10)));
        install(
            &Client::new(),
            "http://127.0.0.1:9/",
            &dir,
            &assets,
            &AtomicBool::new(false),
            |_, _| {},
        )
        .await
        .unwrap();
        remove(&dir).unwrap();
        assert!(!installed(&dir, &assets));
    }

    #[tokio::test]
    async fn rejects_checksum_mismatch_without_leaving_files() {
        let dir = temporary("bad");
        let assets = vec![asset(b"voice data")];
        let (base, server) = serve(vec![b"voice DATA"]);
        let error = install(
            &Client::new(),
            &base,
            &dir,
            &assets,
            &AtomicBool::new(false),
            |_, _| {},
        )
        .await
        .unwrap_err();
        server.join().unwrap();
        assert!(error.contains("failed verification"));
        assert!(!dir.join("test.bin").exists() && !dir.join("test.part").exists());
        remove(&dir).unwrap();
    }

    #[tokio::test]
    async fn reports_offline_and_cancelled_downloads() {
        let dir = temporary("offline");
        let assets = vec![asset(b"voice data")];
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}/", listener.local_addr().unwrap());
        drop(listener);
        let error = install(
            &Client::new(),
            &base,
            &dir,
            &assets,
            &AtomicBool::new(false),
            |_, _| {},
        )
        .await
        .unwrap_err();
        assert!(error.contains("Check your connection"));
        let (base, server) = serve(vec![b"voice data"]);
        let error = install(
            &Client::new(),
            &base,
            &dir,
            &assets,
            &AtomicBool::new(true),
            |_, _| {},
        )
        .await
        .unwrap_err();
        server.join().unwrap();
        assert_eq!(error, "Download cancelled.");
        assert!(!dir.join("test.part").exists());
        remove(&dir).unwrap();
    }

    #[test]
    fn detects_damaged_installations() {
        let dir = temporary("damaged");
        fs::create_dir_all(&dir).unwrap();
        let assets = vec![asset(b"voice data")];
        fs::write(dir.join("test.bin"), b"voice DATA").unwrap();
        assert!(installed(&dir, &assets));
        assert!(verify(&dir, &assets).is_err());
        fs::write(dir.join("test.bin"), b"short").unwrap();
        assert!(!installed(&dir, &assets));
        remove(&dir).unwrap();
    }

    #[test]
    fn pins_every_voice_and_the_model() {
        let assets = assets();
        assert_eq!(assets.len(), 1 + super::super::VOICES.len());
        assert!(assets.iter().all(|asset| asset.sha256.len() == 64));
    }
}
