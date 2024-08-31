use super::*;

impl<T: Config> Pallet<T> {
  // Loosely validates Node ID
  pub fn validate_peer_id(peer_id: PeerId) -> bool {
    let mut valid = false;

    let peer_id_0 = peer_id.0;

    let len = peer_id_0.len();

    // PeerId must be equal to or greater than 32 chars
    // PeerId must be equal to or less than 128 chars
    if len < 32 || len > 128 {
      return false
    };

    let first_char = peer_id_0[0];

    let second_char = peer_id_0[1];

    if first_char == 49 {
      // Node ID (ed25519, using the "identity" multihash) encoded as a raw base58btc multihash
      valid = len <= 128;
    } else if first_char == 81 && second_char == 109 {
      // Node ID (sha256) encoded as a raw base58btc multihash
      valid = len <= 128;
    } else if first_char == 102 || first_char == 98 || first_char == 122 || first_char == 109 {
      // Node ID (sha256) encoded as a CID
      valid = len <= 128;
    }
    
    valid
  }
}