use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

const MAX_MESSAGE_SIZE: usize = 1024 * 1024;

#[derive(Deserialize, Serialize)]
struct Request {
    url: String,
}

fn validate_download_url(raw_url: &str) -> Result<url::Url, String> {
    let parsed = url::Url::parse(raw_url.trim()).map_err(|error| error.to_string())?;
    if !["http", "https"].contains(&parsed.scheme()) || parsed.host_str().is_none() {
        return Err("Only HTTP and HTTPS download URLs are supported".into());
    }
    Ok(parsed)
}

#[derive(Deserialize, Serialize)]
struct Response {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[cfg(windows)]
fn handle_request(request: Request) -> Result<(), String> {
    use std::{
        fs::OpenOptions,
        path::PathBuf,
        process::Command,
        thread,
        time::{Duration, Instant},
    };

    const PIPE: &str = r"\\.\pipe\irani-idm-native-messaging-v1";
    let parsed = validate_download_url(&request.url)?;

    let native_dir: PathBuf = dirs::data_local_dir()
        .ok_or("Could not locate the local application data directory")?
        .join("com.iranidm.desktop")
        .join("native-messaging");
    let app_path = std::fs::read_to_string(native_dir.join("app-path.txt")).map_err(|_| {
        "Irani IDM has not been launched since Native Messaging was installed".to_string()
    })?;
    let app_path = PathBuf::from(app_path.trim());
    if !app_path.is_file() {
        return Err("The Irani IDM application executable could not be found".into());
    }

    let deadline = Instant::now() + Duration::from_secs(12);
    let mut app_started = false;
    let mut pipe = loop {
        match OpenOptions::new().read(true).write(true).open(PIPE) {
            Ok(pipe) => break pipe,
            Err(error) if Instant::now() < deadline => {
                if !app_started {
                    Command::new(&app_path).spawn().map_err(|spawn_error| {
                        format!("Could not start Irani IDM: {spawn_error}")
                    })?;
                    app_started = true;
                }
                let _ = error;
                thread::sleep(Duration::from_millis(150));
            }
            Err(error) => {
                return Err(format!(
                    "Irani IDM did not accept the local connection: {error}"
                ))
            }
        }
    };

    let payload = serde_json::to_vec(&Request {
        url: parsed.to_string(),
    })
    .map_err(|error| error.to_string())?;
    write_frame(&mut pipe, &payload).map_err(|error| error.to_string())?;
    let response = read_frame(&mut pipe).map_err(|error| error.to_string())?;
    let response: Response =
        serde_json::from_slice(&response).map_err(|error| error.to_string())?;
    if response.ok {
        Ok(())
    } else {
        Err(response
            .error
            .unwrap_or_else(|| "Irani IDM rejected the download URL".into()))
    }
}

#[cfg(not(windows))]
fn handle_request(_request: Request) -> Result<(), String> {
    Err("The Irani IDM Native Messaging host is currently supported on Windows only".into())
}

fn read_frame(reader: &mut impl Read) -> io::Result<Vec<u8>> {
    let mut header = [0; 4];
    reader.read_exact(&mut header)?;
    let length = u32::from_le_bytes(header) as usize;
    if length == 0 || length > MAX_MESSAGE_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid native message length",
        ));
    }
    let mut payload = vec![0; length];
    reader.read_exact(&mut payload)?;
    Ok(payload)
}

fn write_frame(writer: &mut impl Write, payload: &[u8]) -> io::Result<()> {
    if payload.is_empty() || payload.len() > MAX_MESSAGE_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid native message length",
        ));
    }
    writer.write_all(&(payload.len() as u32).to_le_bytes())?;
    writer.write_all(payload)?;
    writer.flush()
}

fn main() {
    let result = read_frame(&mut io::stdin())
        .and_then(|bytes| {
            serde_json::from_slice::<Request>(&bytes)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
        })
        .map_err(|error| error.to_string())
        .and_then(handle_request);
    let response = match result {
        Ok(()) => Response {
            ok: true,
            error: None,
        },
        Err(error) => Response {
            ok: false,
            error: Some(error),
        },
    };
    if let Ok(bytes) = serde_json::to_vec(&response) {
        let _ = write_frame(&mut io::stdout(), &bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::{read_frame, validate_download_url, write_frame, MAX_MESSAGE_SIZE};
    use std::io::Cursor;

    #[test]
    fn native_message_frames_round_trip() {
        let payload = br#"{"url":"https://example.test/file.zip"}"#;
        let mut framed = Vec::new();
        write_frame(&mut framed, payload).unwrap();
        assert_eq!(read_frame(&mut Cursor::new(framed)).unwrap(), payload);
    }

    #[test]
    fn native_message_rejects_oversized_frames() {
        let mut header = ((MAX_MESSAGE_SIZE + 1) as u32).to_le_bytes().to_vec();
        header.extend_from_slice(b"x");
        assert!(read_frame(&mut Cursor::new(header)).is_err());
    }

    #[test]
    fn native_host_accepts_only_absolute_http_and_https_urls() {
        assert_eq!(
            validate_download_url("https://example.test/file")
                .unwrap()
                .scheme(),
            "https"
        );
        assert_eq!(
            validate_download_url("http://example.test/file")
                .unwrap()
                .scheme(),
            "http"
        );
        assert!(validate_download_url("file:///C:/secret.txt").is_err());
        assert!(validate_download_url("iranidm://download?url=https://example.test").is_err());
        assert!(validate_download_url("https://").is_err());
    }
}
