use anyhow::{Context, Result, bail};
use std::time::Duration;

// Same servers and query order as astropy's name_resolve: SIMBAD, then NED, then VizieR.
const SESAME_URLS: [&str; 2] = [
    "https://cds.unistra.fr/cgi-bin/nph-sesame/",
    "http://vizier.cfa.harvard.edu/viz-bin/nph-sesame/",
];
const SESAME_DATABASES: &str = "SNV";

pub async fn resolve_position(position: &str) -> Result<String> {
    if looks_like_coordinate_input(position) {
        return Ok(position.trim().to_owned());
    }

    println!(
        "Resolving the coordinates of target {} ...",
        position
    );
    let (ra, dec) = resolve_target(position, &SESAME_URLS).await?;
    println!(
        "Resolved target {} to RA: {}, Dec: {} degrees",
        position, ra, dec
    );
    Ok(format!("{ra},{dec}"))
}

async fn resolve_target(target: &str, base_urls: &[&str]) -> Result<(f64, f64)> {
    let target = target.trim();
    if target.is_empty() {
        bail!("Target name cannot be empty");
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent(concat!("heasarc-hark/", env!("CARGO_PKG_VERSION")))
        .build()
        .context("Failed to create name-resolver client")?;

    let mut errors = Vec::new();
    let mut response = None;
    for base_url in base_urls {
        let url = format!("{base_url}{SESAME_DATABASES}?{}", percent_encode(target));
        match fetch(&client, &url).await {
            Ok(body) => {
                response = Some((url, body));
                break;
            }
            Err(error) => errors.push(format!("{url}: {error:#}")),
        }
    }

    let Some((url, body)) = response else {
        bail!(
            "Name resolutions queries failed. Unable to retrieve coordinates. See errors per URL below:\n{}",
            errors.join("\n\n")
        );
    };

    match parse_response(&body) {
        Some(coordinates) => Ok(coordinates),
        None => bail!("Unable to find coordinates for name '{target}' using {url}"),
    }
}

async fn fetch(client: &reqwest::Client, url: &str) -> Result<String> {
    client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await
        .map_err(Into::into)
}

fn looks_like_coordinate_input(position: &str) -> bool {
    let position = position.trim();
    if position.is_empty() {
        return false;
    }

    let pair = if position.contains(',') {
        position
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
    } else {
        position
            .split_whitespace()
            .collect::<Vec<_>>()
    };

    if pair.len() != 2 {
        return false;
    }

    let Ok(values) = pair
        .iter()
        .map(|value| value.parse::<f64>())
        .collect::<Result<Vec<_>, _>>()
    else {
        return false;
    };

    if values.len() != 2 {
        return false;
    }

    let ra = values[0];
    let dec = values[1];
    (0.0..=360.0).contains(&ra) && (-90.0..=90.0).contains(&dec)
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

/// Extracts the ICRS J2000 position (in degrees) from the `%J ra dec` line of a Sesame response.
fn parse_response(response: &str) -> Option<(f64, f64)> {
    for line in response.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("%J") else {
            continue;
        };

        let mut tokens = rest.split_whitespace();
        let ra = tokens.next()?.parse::<f64>().ok()?;
        let dec = tokens.next()?.parse::<f64>().ok()?;

        if (0.0..=360.0).contains(&ra) && (-90.0..=90.0).contains(&dec) {
            return Some((ra, dec));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{
        looks_like_coordinate_input, parse_response, percent_encode, resolve_position,
        resolve_target,
    };
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpListener;

    #[test]
    fn preserves_valid_coordinate_inputs_and_rejects_invalid_ones() {
        assert!(looks_like_coordinate_input("182.6,39.4"));
        assert!(looks_like_coordinate_input("182.6 39.4"));
        assert!(!looks_like_coordinate_input("182.6,invalid"));
        assert!(!looks_like_coordinate_input("182.6"));
        assert!(!looks_like_coordinate_input("M31"));
        assert!(!looks_like_coordinate_input("NGC 1300"));
    }

    #[tokio::test]
    async fn coordinate_inputs_bypass_name_resolution() {
        assert_eq!(resolve_position("182.6,39.4").await.unwrap(), "182.6,39.4");
        assert_eq!(resolve_position("182.6 39.4").await.unwrap(), "182.6 39.4");
    }

    #[test]
    fn percent_encodes_target_names_for_the_sesame_query() {
        assert_eq!(percent_encode("NGC 1300"), "NGC%201300");
        assert_eq!(percent_encode("Sgr A*"), "Sgr%20A%2A");
    }

    #[test]
    fn parses_sesame_text_response() {
        let response = "# M31\t#Q3\n#=S=Simbad (CDS):    1\n%J 010.684708 +41.268750 = 00:42:44.33 +41:16:07.5\n%I.0 M 31\n";
        assert_eq!(parse_response(response), Some((10.684708, 41.26875)));
        assert_eq!(
            parse_response("%J 83.633 -5.391 = 05:34:32"),
            Some((83.633, -5.391))
        );
    }

    #[test]
    fn rejects_missing_or_out_of_range_coordinates() {
        assert_eq!(parse_response("# nothing found\n#!Simbad: not found"), None);
        assert_eq!(parse_response("%J 361.0 +0.0"), None);
        assert_eq!(parse_response("%J 10.0 +91.0"), None);
    }

    async fn serve_once(status: &str, body: &str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut reader = BufReader::new(socket);
            let mut request_line = String::new();
            reader.read_line(&mut request_line).await.unwrap();
            assert!(request_line.contains("/sesame/SNV?NGC%201300"));

            let mut line = String::new();
            loop {
                line.clear();
                reader.read_line(&mut line).await.unwrap();
                if line == "\r\n" || line.is_empty() {
                    break;
                }
            }
            reader
                .into_inner()
                .write_all(response.as_bytes())
                .await
                .unwrap();
        });
        format!("http://{address}/sesame/")
    }

    #[tokio::test]
    async fn falls_back_to_the_next_server_when_one_fails() {
        let unreachable = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let unreachable_url = format!("http://{}/sesame/", unreachable.local_addr().unwrap());
        drop(unreachable);
        let broken = serve_once("500 Internal Server Error", "oops").await;
        let working = serve_once("200 OK", "%J 10.5 -20.25 = 00:42:00 -20:15:00\n").await;

        let urls = [unreachable_url.as_str(), broken.as_str(), working.as_str()];
        assert_eq!(
            resolve_target("NGC 1300", &urls).await.unwrap(),
            (10.5, -20.25)
        );
    }

    #[tokio::test]
    async fn reports_every_server_error_when_all_fail() {
        let unreachable = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let unreachable_url = format!("http://{}/sesame/", unreachable.local_addr().unwrap());
        drop(unreachable);
        let broken = serve_once("500 Internal Server Error", "oops").await;

        let urls = [unreachable_url.as_str(), broken.as_str()];
        let error = resolve_target("NGC 1300", &urls).await.unwrap_err();
        let message = error.to_string();
        assert!(message.contains("Name resolutions queries failed"));
        assert!(message.contains(&unreachable_url));
        assert!(message.contains(&broken));
    }

    #[tokio::test]
    async fn reports_unknown_names() {
        let server = serve_once("200 OK", "#!Simbad: Not found\n").await;
        let error = resolve_target("NGC 1300", &[server.as_str()])
            .await
            .unwrap_err();
        assert!(error.to_string().contains("Unable to find coordinates"));
    }
}
