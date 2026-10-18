//! DTV (<https://dtv.gg>): every salt sent to deadlock-api also goes to DTV, under the
//! player's personal ingest token. With a match's salts, DTV downloads its metadata straight
//! from Valve's replay servers — the tournament matches (private lobbies) deadlock-api
//! doesn't keep, without spending anyone's quota.
//!
//! Best effort: no token, DTV down or a token revoked never stops the deadlock-api upload.

use core::time::Duration;
use std::collections::HashSet;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::thread::sleep;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};
use ureq::Error::StatusCode;

use crate::paths;
use crate::utils::{MAX_MATCH_ID, Salts, http_client};

const DEFAULT_URL: &str = "https://api.dtv.gg/api/ingest";
/// DTV takes at most this many salts per request.
const MAX_PER_REQUEST: usize = 500;
const MAX_ATTEMPTS: u32 = 3;

/// `DTV_INGEST_URL` points the app at another DTV (preview, local stack).
fn base_url() -> String {
    std::env::var("DTV_INGEST_URL")
        .ok()
        .filter(|url| !url.trim().is_empty())
        .map_or_else(
            || DEFAULT_URL.to_owned(),
            |url| url.trim().trim_end_matches('/').to_owned(),
        )
}

fn token_path() -> Option<PathBuf> {
    Some(paths::data_dir()?.join("dtv-token"))
}

/// The ingest token from `DTV_INGEST_TOKEN`, else the one saved from the status window or
/// `--dtv-token`. Read on every upload, so a token saved while the watcher runs is used at
/// once.
pub(crate) fn token() -> Option<String> {
    std::env::var("DTV_INGEST_TOKEN")
        .ok()
        .or_else(|| std::fs::read_to_string(token_path()?).ok())
        .map(|token| token.trim().to_owned())
        .filter(|token| !token.is_empty())
}

pub(crate) fn save_token(token: &str) -> Result<(), String> {
    let path = token_path().ok_or("Cannot locate the app's data folder")?;
    std::fs::write(&path, token.trim()).map_err(|e| format!("Cannot save the token: {e}"))
}

pub(crate) fn clear_token() -> Result<(), String> {
    let Some(path) = token_path() else {
        return Ok(());
    };
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("Cannot remove the token: {e}")),
    }
}

/// What DTV needs to fetch a match's metadata from Valve.
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Entry {
    match_id: u64,
    cluster_id: u32,
    metadata_salt: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    replay_salt: Option<u32>,
}

/// The salts that can give a match's metadata; a replay salt alone can't, so it's left out.
fn entries<'a>(salts: impl IntoIterator<Item = &'a Salts>) -> Vec<Entry> {
    let mut seen = HashSet::new();
    salts
        .into_iter()
        .filter_map(|s| {
            Some(Entry {
                match_id: s.match_id,
                cluster_id: s.cluster_id?,
                metadata_salt: s.metadata_salt.filter(|&salt| salt != 0)?,
                replay_salt: s.replay_salt,
            })
        })
        .filter(|e| e.match_id <= MAX_MATCH_ID && seen.insert(e.match_id))
        .collect()
}

/// Sends `salts` to DTV if a token is set. Blocking; failures are logged, never returned.
pub(crate) fn send(salts: &[Salts]) {
    let Some(token) = token() else {
        return;
    };
    let entries = entries(salts);
    for batch in entries.chunks(MAX_PER_REQUEST) {
        if let Err(e) = post(&token, batch) {
            warn!("DTV: {} salts not sent: {e}", batch.len());
            return;
        }
        debug!("DTV: {} salts sent", batch.len());
    }
}

fn post(token: &str, batch: &[Entry]) -> Result<(), String> {
    let url = format!("{}/salts", base_url());
    let mut attempt = 0;
    loop {
        attempt += 1;
        let response = http_client()
            .post(&url)
            .header("Authorization", &format!("Bearer {token}"))
            .send_json(batch);
        match response {
            Ok(_) => return Ok(()),
            Err(StatusCode(401 | 403)) => {
                return Err("the DTV token was refused (revoked?). Link the app again.".into());
            }
            Err(StatusCode(400)) => return Err("DTV rejected the request".into()),
            Err(e) if attempt >= MAX_ATTEMPTS => return Err(e.to_string()),
            Err(_) => sleep(Duration::from_secs(3)),
        }
    }
}

#[derive(Deserialize)]
struct Me {
    username: String,
}

/// The DTV account `token` belongs to.
pub(crate) fn whoami(token: &str) -> Result<String, String> {
    let response = http_client()
        .get(&format!("{}/me", base_url()))
        .header("Authorization", &format!("Bearer {}", token.trim()))
        .call();
    match response {
        Ok(mut response) => response
            .body_mut()
            .read_json::<Me>()
            .map(|me| me.username)
            .map_err(|e| format!("Unexpected answer from DTV: {e}")),
        Err(StatusCode(401 | 403)) => {
            Err("Unknown or revoked token. Copy it again from your DTV profile.".into())
        }
        Err(e) => Err(format!("Cannot reach DTV: {e}")),
    }
}

/// One line of the salts log (`ingestion_cache`), as far as DTV cares.
#[derive(Deserialize)]
struct LoggedSalts {
    match_id: u64,
    cluster_id: Option<u32>,
    metadata_salt: Option<u32>,
    replay_salt: Option<u32>,
}

/// Sends every salt already in the local log: those submitted before the app was linked to
/// DTV were only sent to deadlock-api. Returns how many were sent.
pub(crate) fn send_logged() -> usize {
    let Some(token) = token() else {
        return 0;
    };
    let Some(file) = paths::salts_log_path().and_then(|p| std::fs::File::open(p).ok()) else {
        return 0;
    };
    let logged: Vec<Salts> = BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter_map(|line| serde_json::from_str::<LoggedSalts>(&line).ok())
        .map(|l| Salts {
            match_id: l.match_id,
            cluster_id: l.cluster_id,
            metadata_salt: l.metadata_salt,
            replay_salt: l.replay_salt,
            username: None,
        })
        .collect();
    let entries = entries(&logged);
    let mut sent = 0;
    for batch in entries.chunks(MAX_PER_REQUEST) {
        if let Err(e) = post(&token, batch) {
            warn!("DTV: past matches not sent: {e}");
            break;
        }
        sent += batch.len();
    }
    info!("DTV: {sent} past matches sent");
    sent
}

#[cfg(test)]
mod tests {
    use super::*;

    fn salts(match_id: u64, cluster: Option<u32>, metadata: Option<u32>) -> Salts {
        Salts {
            match_id,
            cluster_id: cluster,
            metadata_salt: metadata,
            replay_salt: None,
            username: None,
        }
    }

    #[test]
    fn only_salts_that_give_metadata_are_sent() {
        let all = [
            salts(1, Some(405), Some(42)),
            salts(2, Some(405), None),
            salts(3, None, Some(42)),
            salts(4, Some(405), Some(0)),
            salts(1, Some(405), Some(42)),
            salts(MAX_MATCH_ID + 1, Some(405), Some(42)),
        ];
        let ids: Vec<u64> = entries(&all).iter().map(|e| e.match_id).collect();
        assert_eq!(ids, [1]);
    }

    #[test]
    fn entries_serialize_like_deadlock_api() {
        let json = serde_json::to_string(&entries(&[salts(7, Some(271), Some(9))])).unwrap();
        assert_eq!(
            json,
            r#"[{"match_id":7,"cluster_id":271,"metadata_salt":9}]"#
        );
    }
}
