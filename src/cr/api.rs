use reqwest::blocking::Client as HttpClient;
use reqwest::header::{
    HeaderMap, HeaderValue, ACCEPT_LANGUAGE, AUTHORIZATION, CONTENT_TYPE, USER_AGENT,
};

use super::models::{BattleLogEntry, Player};
use crate::core::constants::CR_API_BASE;
use crate::core::util::normalize_tag;

pub fn build_http_client(user_agent: &str, api_key: &str) -> HttpClient {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {api_key}")).unwrap(),
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(USER_AGENT, HeaderValue::from_str(user_agent).unwrap());
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("es"));

    HttpClient::builder()
        .default_headers(headers)
        .build()
        .expect("No se pudo crear el cliente HTTP")
}

pub fn fetch_player(http: &HttpClient, player_tag: &str) -> Result<Player, reqwest::Error> {
    let cleaned = normalize_tag(player_tag);
    let url = format!("{CR_API_BASE}/v1/players/%23{cleaned}");
    http.get(url).send()?.error_for_status()?.json::<Player>()
}

pub fn fetch_last_battle(
    http: &HttpClient,
    player_tag: &str,
) -> Result<Option<BattleLogEntry>, reqwest::Error> {
    let cleaned = normalize_tag(player_tag);
    let url = format!("{CR_API_BASE}/v1/players/%23{cleaned}/battlelog");
    let list = http
        .get(url)
        .send()?
        .error_for_status()?
        .json::<Vec<BattleLogEntry>>()?;

    Ok(list.into_iter().next())
}

pub fn probe_image_url(
    client: &HttpClient,
    url: &str,
) -> (i32, Option<String>, Option<String>, Option<String>) {
    use reqwest::header::CONTENT_LENGTH;

    match client.head(url).send() {
        Ok(resp) => {
            let status = resp.status().as_u16() as i32;
            let ct = resp
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());
            let cl = resp
                .headers()
                .get(CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());
            (status, ct, cl, None)
        }
        Err(e) => (-1, None, None, Some(e.to_string())),
    }
}
