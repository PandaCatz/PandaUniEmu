use sha2::{Digest, Sha256};
use std::fmt::{Display, Formatter};

pub(crate) const FIXTURE_ID: &str = "kevin-horton-v1.00";
pub(crate) const EXPECTED_ROWS: usize = 8_991;
pub(crate) const EXPECTED_TRANSITIONS: usize = 8_990;
pub(crate) const ROM_BYTES: usize = 24_592;
pub(crate) const ROM_SHA256: &str =
    "f67d55fd6b3cf0bad1cc85f1df0d739c65b53e79cecb7fea8f77ec0eadab0004";
const QMT_LOG_BYTES: usize = 868_158;
const QMT_LOG_SHA256: &str = "627c8e180b1a924dfa705c5dc6958fad7ab75a62de556173caf880ccc1337540";
const PINNED_LOG_BYTES: usize = 859_167;
const PINNED_LOG_SHA256: &str = "442c4dd5539c7e88b3fd73c7b732a7eadbd22b47c2cd9e58397ef147f64f6f8f";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LogVariant {
    QmtCrLf,
    PinnedLf,
}

impl LogVariant {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::QmtCrLf => "qmt-crlf",
            Self::PinnedLf => "pinned-lf",
        }
    }

    pub(crate) const fn bytes(self) -> usize {
        match self {
            Self::QmtCrLf => QMT_LOG_BYTES,
            Self::PinnedLf => PINNED_LOG_BYTES,
        }
    }

    pub(crate) const fn sha256(self) -> &'static str {
        match self {
            Self::QmtCrLf => QMT_LOG_SHA256,
            Self::PinnedLf => PINNED_LOG_SHA256,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct AcceptedIdentity {
    pub(crate) log_variant: LogVariant,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum IdentityFailure {
    RomMismatch,
    ReferenceLogMismatch,
}

impl Display for IdentityFailure {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RomMismatch => formatter.write_str("ROM identity mismatch"),
            Self::ReferenceLogMismatch => formatter.write_str("reference-log identity mismatch"),
        }
    }
}

pub(crate) fn verify(image: &[u8], reference: &[u8]) -> Result<AcceptedIdentity, IdentityFailure> {
    if image.len() != ROM_BYTES {
        return Err(IdentityFailure::RomMismatch);
    }
    let rom_sha256 = sha256_hex(image);
    if rom_sha256 != ROM_SHA256 {
        return Err(IdentityFailure::RomMismatch);
    }
    if !matches!(reference.len(), QMT_LOG_BYTES | PINNED_LOG_BYTES) {
        return Err(IdentityFailure::ReferenceLogMismatch);
    }
    let log_sha256 = sha256_hex(reference);
    classify(image.len(), &rom_sha256, reference.len(), &log_sha256)
}

fn classify(
    rom_bytes: usize,
    rom_sha256: &str,
    log_bytes: usize,
    log_sha256: &str,
) -> Result<AcceptedIdentity, IdentityFailure> {
    if rom_bytes != ROM_BYTES || rom_sha256 != ROM_SHA256 {
        return Err(IdentityFailure::RomMismatch);
    }
    let log_variant = match (log_bytes, log_sha256) {
        (QMT_LOG_BYTES, QMT_LOG_SHA256) => LogVariant::QmtCrLf,
        (PINNED_LOG_BYTES, PINNED_LOG_SHA256) => LogVariant::PinnedLf,
        _ => return Err(IdentityFailure::ReferenceLogMismatch),
    };
    Ok(AcceptedIdentity { log_variant })
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_the_published_abc_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn classifier_accepts_only_the_two_exact_pair_records() {
        assert_eq!(
            classify(ROM_BYTES, ROM_SHA256, QMT_LOG_BYTES, QMT_LOG_SHA256),
            Ok(AcceptedIdentity {
                log_variant: LogVariant::QmtCrLf,
            })
        );
        assert_eq!(
            classify(ROM_BYTES, ROM_SHA256, PINNED_LOG_BYTES, PINNED_LOG_SHA256),
            Ok(AcceptedIdentity {
                log_variant: LogVariant::PinnedLf,
            })
        );
    }

    #[test]
    fn classifier_rejects_size_hash_and_crossed_metadata() {
        assert_eq!(
            classify(ROM_BYTES - 1, ROM_SHA256, QMT_LOG_BYTES, QMT_LOG_SHA256),
            Err(IdentityFailure::RomMismatch)
        );
        assert_eq!(
            classify(
                ROM_BYTES,
                &format!("0{}", &ROM_SHA256[1..]),
                QMT_LOG_BYTES,
                QMT_LOG_SHA256
            ),
            Err(IdentityFailure::RomMismatch)
        );
        assert_eq!(
            classify(ROM_BYTES, ROM_SHA256, QMT_LOG_BYTES, PINNED_LOG_SHA256),
            Err(IdentityFailure::ReferenceLogMismatch)
        );
        assert_eq!(
            classify(ROM_BYTES, ROM_SHA256, PINNED_LOG_BYTES, QMT_LOG_SHA256),
            Err(IdentityFailure::ReferenceLogMismatch)
        );
    }

    #[test]
    fn generated_bytes_cannot_claim_the_reviewed_identity() {
        assert_eq!(
            verify(&vec![0; ROM_BYTES], &vec![0; QMT_LOG_BYTES]),
            Err(IdentityFailure::RomMismatch)
        );
    }
}
