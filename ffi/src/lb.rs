use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use rand::seq::SliceRandom;

use crate::dsn::{Config, Endpoint};

/// How long a server that refused a connection is skipped.
const DOWN_FOR: Duration = Duration::from_secs(30);

struct Discovered {
    endpoints: Vec<Endpoint>,
    expires: Instant,
}

fn discovered() -> &'static Mutex<HashMap<String, Discovered>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Discovered>>> = OnceLock::new();
    CACHE.get_or_init(Default::default)
}

fn down() -> &'static Mutex<HashMap<(String, u16), Instant>> {
    static DOWN: OnceLock<Mutex<HashMap<(String, u16), Instant>>> = OnceLock::new();
    DOWN.get_or_init(Default::default)
}

/// Process-wide key of a cluster: the configured seed endpoints.
pub fn cluster_key(cfg: &Config) -> String {
    cfg.endpoints.iter().map(|e| format!("{}:{}", e.host, e.port)).collect::<Vec<_>>().join(",")
}

/// Candidate endpoints in connection order: live ones shuffled, then recently failed ones as a last resort.
pub fn candidates(cfg: &Config) -> Vec<Endpoint> {
    let mut list = cfg.endpoints.clone();
    if cfg.discover
        && let Ok(cache) = discovered().lock()
        && let Some(d) = cache.get(&cluster_key(cfg)).filter(|d| d.expires > Instant::now() && !d.endpoints.is_empty())
    {
        list = d.endpoints.clone();
    }
    list.shuffle(&mut rand::rng());
    let now = Instant::now();
    let failed: Vec<bool> = match down().lock() {
        Ok(map) => list.iter().map(|e| map.get(&(e.host.clone(), e.port)).is_some_and(|&until| until > now)).collect(),
        Err(_) => vec![false; list.len()],
    };
    let (mut live, dead): (Vec<_>, Vec<_>) = list.into_iter().zip(failed).partition(|(_, f)| !f);
    live.extend(dead);
    live.into_iter().map(|(e, _)| e).collect()
}

pub fn mark_down(endpoint: &Endpoint) {
    if let Ok(mut map) = down().lock() {
        map.insert((endpoint.host.clone(), endpoint.port), Instant::now() + DOWN_FOR);
    }
}

pub fn mark_up(endpoint: &Endpoint) {
    if let Ok(mut map) = down().lock() {
        map.remove(&(endpoint.host.clone(), endpoint.port));
    }
}

pub fn needs_discovery(cfg: &Config) -> bool {
    cfg.discover && discovered().lock().map(|c| c.get(&cluster_key(cfg)).is_none_or(|d| d.expires <= Instant::now())).unwrap_or(false)
}

pub fn store(cfg: &Config, endpoints: Vec<Endpoint>) {
    if let Ok(mut cache) = discovered().lock() {
        cache.insert(cluster_key(cfg), Discovered { endpoints, expires: Instant::now() + cfg.discover_ttl });
    }
}

pub fn forget(cfg: &Config) {
    if let Ok(mut cache) = discovered().lock() {
        cache.remove(&cluster_key(cfg));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_endpoints_go_last() {
        let cfg = Config::parse("host=lb-a:1,lb-b:2,lb-c:3", None, None, &[]).unwrap();
        mark_down(&Endpoint { host: "lb-b".into(), port: 2 });
        for _ in 0..20 {
            assert_eq!(candidates(&cfg).last().unwrap().host, "lb-b");
        }
        mark_up(&Endpoint { host: "lb-b".into(), port: 2 });
    }

    #[test]
    fn discovery_replaces_seeds_until_it_expires() {
        let mut cfg = Config::parse("host=seed-x:4000;discover=1", None, None, &[]).unwrap();
        assert!(needs_discovery(&cfg));
        store(&cfg, vec![Endpoint { host: "10.0.0.1".into(), port: 4000 }]);
        assert!(!needs_discovery(&cfg));
        assert_eq!(candidates(&cfg)[0].host, "10.0.0.1");
        forget(&cfg);
        cfg.discover = false;
        assert_eq!(candidates(&cfg)[0].host, "seed-x");
    }
}
