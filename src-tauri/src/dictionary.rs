use crate::model::Entry;
use reqwest::{Client, StatusCode, Url};
use std::time::Duration;

pub struct Dictionary {
    client: Client,
}
#[derive(Debug)]
pub struct LookupError {
    pub kind: &'static str,
    pub message: String,
}
impl Dictionary {
    pub fn new() -> Result<Self, String> {
        let client = Client::builder()
            .user_agent("DIO/0.1 (desktop dictionary)")
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self { client })
    }
    pub async fn lookup(&self, word: &str) -> Result<Vec<Entry>, LookupError> {
        self.fetch(entry_url(word)).await
    }
    async fn fetch(&self, url: Url) -> Result<Vec<Entry>, LookupError> {
        let mut response = self
            .client
            .get(url.clone())
            .send()
            .await
            .map_err(network_error)?;
        for delay in [150, 400] {
            if !matches!(
                response.status(),
                StatusCode::BAD_GATEWAY
                    | StatusCode::SERVICE_UNAVAILABLE
                    | StatusCode::GATEWAY_TIMEOUT
            ) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(delay)).await;
            response = self
                .client
                .get(url.clone())
                .send()
                .await
                .map_err(network_error)?;
        }
        let status = response.status();
        if status == StatusCode::NOT_FOUND {
            return Err(LookupError {
                kind: "notFound",
                message: "No entry found. Try another form of the word.".into(),
            });
        }
        if !status.is_success() {
            return Err(LookupError {
                kind: "network",
                message: if status == StatusCode::TOO_MANY_REQUESTS {
                    "The dictionary is busy. Try again shortly.".into()
                } else if matches!(
                    status,
                    StatusCode::BAD_GATEWAY
                        | StatusCode::SERVICE_UNAVAILABLE
                        | StatusCode::GATEWAY_TIMEOUT
                ) {
                    "The dictionary is temporarily unavailable. Try again shortly.".into()
                } else {
                    format!("Dictionary service returned {status}.")
                },
            });
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(network_error)? {
            if body.len() + chunk.len() > 2_000_000 {
                return Err(LookupError {
                    kind: "network",
                    message: "Dictionary response exceeded the size limit.".into(),
                });
            }
            body.extend_from_slice(&chunk);
        }
        let entries: Vec<Entry> = serde_json::from_slice(&body).map_err(|_| LookupError {
            kind: "network",
            message: "The dictionary returned an unreadable entry.".into(),
        })?;
        if entries.is_empty()
            || !entries
                .iter()
                .any(|e| e.meanings.iter().any(|m| !m.definitions.is_empty()))
        {
            return Err(LookupError {
                kind: "notFound",
                message: "No definitions available for this selection.".into(),
            });
        }
        Ok(entries)
    }
    pub async fn audio(&self, address: &str) -> Result<Vec<u8>, String> {
        let url = validate_audio_url(address)?;
        let mut response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?;
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if !content_type.starts_with("audio/") && content_type != "application/ogg" {
            return Err("Unsupported pronunciation format.".into());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
            if bytes.len() + chunk.len() > 5_000_000 {
                return Err("Pronunciation file is too large.".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }
}
fn entry_url(word: &str) -> Url {
    let mut url =
        Url::parse("https://api.dictionaryapi.dev/api/v2/entries/en/").expect("fixed URL");
    url.path_segments_mut()
        .expect("base URL")
        .pop_if_empty()
        .push(word);
    url
}
fn network_error(error: reqwest::Error) -> LookupError {
    LookupError {
        kind: if error.is_timeout() {
            "timeout"
        } else {
            "network"
        },
        message: if error.is_timeout() {
            "The dictionary request timed out.".into()
        } else {
            "Cannot reach the dictionary. Check your connection and try again.".into()
        },
    }
}
fn validate_audio_url(address: &str) -> Result<Url, String> {
    let address = if address.starts_with("//") {
        format!("https:{address}")
    } else {
        address.to_string()
    };
    let url = Url::parse(&address).map_err(|_| "Invalid audio address.")?;
    let host = url.host_str().unwrap_or("");
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port_or_known_default() != Some(443)
        || ![
            "api.dictionaryapi.dev",
            "ssl.gstatic.com",
            "upload.wikimedia.org",
        ]
        .contains(&host)
    {
        return Err("This pronunciation host is not supported.".into());
    }
    Ok(url)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lookup_url_has_one_separator_before_word() {
        assert_eq!(
            entry_url("another").as_str(),
            "https://api.dictionaryapi.dev/api/v2/entries/en/another"
        );
        assert_eq!(
            entry_url("ice cream").as_str(),
            "https://api.dictionaryapi.dev/api/v2/entries/en/ice%20cream"
        );
    }
    #[test]
    fn audio_destinations_are_restricted() {
        assert!(validate_audio_url("//ssl.gstatic.com/sound.mp3").is_ok());
        for url in [
            "http://ssl.gstatic.com/x",
            "https://localhost/x",
            "https://upload.wikimedia.org.evil.test/x",
            "https://user@ssl.gstatic.com/x",
        ] {
            assert!(validate_audio_url(url).is_err());
        }
    }
    #[tokio::test]
    async fn handles_provider_failures() {
        use std::{
            io::{Read, Write},
            net::TcpListener,
        };
        for (status, body, expected) in [
            ("404 Not Found", "{}", "notFound"),
            ("429 Too Many Requests", "{}", "network"),
            ("200 OK", "not json", "network"),
            ("200 OK", "[]", "notFound"),
        ] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let payload = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let server = std::thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                let mut request = [0; 4096];
                let mut read = 0;
                while !request[..read].windows(4).any(|part| part == b"\r\n\r\n") {
                    let count = socket.read(&mut request[read..]).unwrap();
                    assert!(count > 0);
                    read += count;
                }
                socket.write_all(payload.as_bytes()).unwrap();
            });
            let error = Dictionary::new()
                .unwrap()
                .fetch(Url::parse(&format!("http://{address}")).unwrap())
                .await
                .unwrap_err();
            assert_eq!(error.kind, expected, "{status}: {}", error.message);
            server.join().unwrap();
        }
    }
    #[tokio::test]
    async fn retries_gateway_errors() {
        use std::{
            io::{Read, Write},
            net::TcpListener,
        };
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            for (status, body) in [
                ("502 Bad Gateway", ""),
                ("503 Service Unavailable", ""),
                (
                    "200 OK",
                    r#"[{"word":"test","meanings":[{"definitions":[{"definition":"A trial."}]}]}]"#,
                ),
            ] {
                let (mut socket, _) = listener.accept().unwrap();
                let mut request = [0; 4096];
                let _ = socket.read(&mut request).unwrap();
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                socket.write_all(response.as_bytes()).unwrap();
            }
        });
        let entries = Dictionary::new()
            .unwrap()
            .fetch(Url::parse(&format!("http://{address}")).unwrap())
            .await
            .unwrap();
        assert_eq!(entries[0].word, "test");
        server.join().unwrap();
    }
    #[tokio::test]
    async fn delayed_response_times_out() {
        use std::{
            io::{Read, Write},
            net::TcpListener,
        };
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = [0; 4096];
            let _ = socket.read(&mut request);
            std::thread::sleep(Duration::from_millis(100));
            let _ = socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n[]");
        });
        let dictionary = Dictionary {
            client: Client::builder()
                .timeout(Duration::from_millis(20))
                .build()
                .unwrap(),
        };
        let error = dictionary
            .fetch(Url::parse(&format!("http://{address}")).unwrap())
            .await
            .unwrap_err();
        assert_eq!(error.kind, "timeout");
        server.join().unwrap();
    }
    #[tokio::test]
    async fn unavailable_provider_reports_network_error() {
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);
        let error = Dictionary::new()
            .unwrap()
            .fetch(Url::parse(&format!("http://{address}")).unwrap())
            .await
            .unwrap_err();
        assert_eq!(error.kind, "network");
    }
}
