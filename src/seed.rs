use std::time::{SystemTime, UNIX_EPOCH};

/// Graine tirée de l'horloge système, pour démarrer une partie différente à chaque fois.
/// Renvoie 0 si l'horloge est avant 1970 (ce qui ne devrait pas arriver).
pub fn from_clock() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}
