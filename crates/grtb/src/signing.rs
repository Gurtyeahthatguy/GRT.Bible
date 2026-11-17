//! Signatures of `.grtb` packs: Ed25519 over the content digest, kept in the meta table.

use std::path::Path;

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::module::{content_digest, Module};

/// Keys whose packs the library names, by public key in hex.
pub const TRUSTED: [(&str, &str); 1] = [("GRT Project", "4f4d552e5af3eacd3e63e03dfcbd45e578012a57368d95060cafcb4eee9b038f")];

const CONTEXT: &[u8] = b"GRT Bible module\0";

/// What the library says about a module's signature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum Seal {
    Unsigned,
    /// Signed by a key the program knows.
    Signed { by: String, key: String },
    /// Signed, by a key the program does not know.
    Unknown { key: String },
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex<const N: usize>(text: &str) -> Option<[u8; N]> {
    if text.len() != N * 2 || !text.is_ascii() {
        return None;
    }
    let mut out = [0u8; N];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

/// A short form of a public key, for people to compare.
pub fn fingerprint(key: &str) -> String {
    key.chars().take(16).collect::<Vec<_>>().chunks(4).map(|c| c.iter().collect::<String>()).collect::<Vec<_>>().join(" ")
}

fn message(digest: &[u8; 32]) -> Vec<u8> {
    [CONTEXT, digest.as_slice()].concat()
}

/// Checks a module's signature; a signature that does not match the content is an error.
pub fn check(module: &Module) -> Result<Seal, String> {
    let Some(value) = module.meta("signature")? else { return Ok(Seal::Unsigned) };
    let mut parts = value.split(' ');
    let (Some("ed25519"), Some(key), Some(sig), None) = (parts.next(), parts.next(), parts.next(), parts.next()) else {
        return Err("the module carries a signature in a form this version does not read".into());
    };
    let public = unhex::<32>(key).and_then(|k| VerifyingKey::from_bytes(&k).ok()).ok_or("the module's signing key is not valid")?;
    let signature = unhex::<64>(sig).map(|s| Signature::from_bytes(&s)).ok_or("the module's signature is not valid")?;
    public
        .verify(&message(&module.digest()?), &signature)
        .map_err(|_| "the module was changed after it was signed".to_string())?;
    let key = key.to_string();
    Ok(match TRUSTED.iter().find(|(_, k)| *k == key) {
        Some((by, _)) => Seal::Signed { by: by.to_string(), key },
        None => Seal::Unknown { key },
    })
}

/// The public key of a secret seed, in hex.
pub fn public_key(seed: &[u8; 32]) -> String {
    hex(SigningKey::from_bytes(seed).verifying_key().as_bytes())
}

/// Signs a module file in place with a secret seed, replacing any earlier signature.
pub fn sign(path: &Path, seed: &[u8; 32]) -> Result<String, String> {
    let db = Connection::open(path).map_err(|e| e.to_string())?;
    db.execute("DELETE FROM meta WHERE key = 'signature'", []).map_err(|e| e.to_string())?;
    let key = SigningKey::from_bytes(seed);
    let signature = key.sign(&message(&content_digest(&db)?));
    let value = format!("ed25519 {} {}", hex(key.verifying_key().as_bytes()), hex(&signature.to_bytes()));
    db.execute("INSERT INTO meta (key, value) VALUES ('signature', ?1)", params![value]).map_err(|e| e.to_string())?;
    db.execute_batch("VACUUM").map_err(|e| e.to_string())?;
    Ok(hex(key.verifying_key().as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assemble::{assemble_resource, Description};
    use crate::module::{write, EntryContent};
    use crate::refs::Ref;

    fn pack(dir: &Path) -> std::path::PathBuf {
        let description = Description {
            id: "test".into(),
            abbreviation: "TST".into(),
            title: "Test".into(),
            language: "en".into(),
            license: "Public domain".into(),
            ..Default::default()
        };
        let r = Ref::parse("John.3.16").unwrap();
        let entries = vec![EntryContent { key: "a".into(), title: "A".into(), body: "<p>Love.</p>".into(), spans: vec![(r, r)], ..Default::default() }];
        let content = assemble_resource(&description, "commentary", entries, Vec::new(), Vec::new(), "2026-11-14", "");
        let path = dir.join("TST.grtb");
        write(&path, &content).unwrap();
        path
    }

    #[test]
    fn a_signed_pack_says_by_whom_and_a_changed_one_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = pack(dir.path());
        assert_eq!(check(&Module::open(&path).unwrap()).unwrap(), Seal::Unsigned);
        let seed = [7u8; 32];
        let key = sign(&path, &seed).unwrap();
        assert_eq!(key, public_key(&seed));
        assert_eq!(check(&Module::open(&path).unwrap()).unwrap(), Seal::Unknown { key: key.clone() });
        // A pack built and signed again from the same content is the same file.
        let again = tempfile::tempdir().unwrap();
        let rebuilt = pack(again.path());
        sign(&rebuilt, &seed).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), std::fs::read(&rebuilt).unwrap());
        let db = Connection::open(&path).unwrap();
        db.execute("UPDATE entries SET body = '<p>Hate.</p>'", []).unwrap();
        drop(db);
        let refused = check(&Module::open(&path).unwrap()).unwrap_err();
        assert!(refused.contains("changed after it was signed"), "{refused}");
    }

    #[test]
    fn a_signature_cannot_be_moved_to_another_pack() {
        let dir = tempfile::tempdir().unwrap();
        let path = pack(dir.path());
        sign(&path, &[9u8; 32]).unwrap();
        let signature: String = Connection::open(&path).unwrap().query_row("SELECT value FROM meta WHERE key = 'signature'", [], |r| r.get(0)).unwrap();
        let other = tempfile::tempdir().unwrap();
        let second = pack(other.path());
        let db = Connection::open(&second).unwrap();
        db.execute("UPDATE meta SET value = 'Other' WHERE key = 'title'", []).unwrap();
        db.execute("INSERT INTO meta (key, value) VALUES ('signature', ?1)", params![signature]).unwrap();
        drop(db);
        assert!(check(&Module::open(&second).unwrap()).is_err());
        assert_eq!(fingerprint("0123456789abcdef0123"), "0123 4567 89ab cdef");
    }
}
