//! Standard base64 with padding, as Go's `base64.StdEncoding` writes and
//! reads it: for the clipboard (OSC 52).

/// The standard alphabet.
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// `bytes` in base64, padded to whole groups of four.
pub(crate) fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let mut group = [0u8; 3];
        group[..chunk.len()].copy_from_slice(chunk);
        let v = u32::from_be_bytes([0, group[0], group[1], group[2]]);
        for i in 0..4 {
            if i <= chunk.len() {
                let index = (v >> (18 - 6 * i)) & 0x3F;
                out.push(char::from(ALPHABET[usize::try_from(index).unwrap_or(0)]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// The value of one base64 character.
fn value(c: u8) -> Option<u32> {
    let i = ALPHABET.iter().position(|&a| a == c)?;
    u32::try_from(i).ok()
}

/// Decodes base64 as Go's `StdEncoding.DecodeString` does: line breaks
/// are skipped, padding is required, anything else outside the alphabet
/// is an error (`None`).
pub(crate) fn decode(text: &[u8]) -> Option<Vec<u8>> {
    let text: Vec<u8> = text
        .iter()
        .copied()
        .filter(|&b| b != b'\r' && b != b'\n')
        .collect();
    if text.len() % 4 != 0 {
        return None;
    }
    let groups = text.len() / 4;
    let mut out = Vec::with_capacity(groups * 3);
    for (i, group) in text.chunks(4).enumerate() {
        out.extend_from_slice(&group_bytes(group, i + 1 == groups)?);
    }
    Some(out)
}

/// The bytes four characters stand for; padding only in the `last`
/// group.
fn group_bytes(group: &[u8], last: bool) -> Option<Vec<u8>> {
    let pad = group.iter().rev().take_while(|&&b| b == b'=').count();
    if pad > 2 || (pad > 0 && !last) {
        return None;
    }
    let mut v = 0;
    for &c in &group[..4 - pad] {
        v = v << 6 | value(c)?;
    }
    v <<= 6 * pad;
    Some(v.to_be_bytes()[1..4 - pad].to_vec())
}

#[cfg(test)]
mod tests;
