//! Ingestion library for folio.
//!
//! Currently a stub: the ingest logic lives in `folio-cli` and `folio-core`
//! and moves here as the library API (`IngestConfig`, `Ingester`) when
//! story 005 (incremental ingestion) begins. See the roadmap's
//! architecture direction — this crate becomes the application layer.

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        // 2 + 2 == 2 * 2, so use operands that distinguish addition
        assert_eq!(add(2, 3), 5);
    }
}
