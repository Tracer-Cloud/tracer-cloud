use once_cell::sync::Lazy;
use rand::seq::SliceRandom;
use rand::Rng;
use sha2::{Digest, Sha256};

static ADJECTIVES: Lazy<Vec<&str>> =
    Lazy::new(|| vec!["snowy", "silent", "desert", "mystic", "ancient"]);
static ANIMALS: Lazy<Vec<&str>> = Lazy::new(|| {
    vec![
        "owl", "wolf", "lion", "tiger", "hawk", "eagle", "fox", "bear", "penguin", "dolphin",
        "elephant", "leopard", "giraffe", "rhino", "panther", "falcon", "lynx", "moose", "otter",
        "raccoon",
    ]
});

pub(super) fn generate_run_name() -> String {
    let mut rng = rand::thread_rng();
    let adjective = ADJECTIVES.choose(&mut rng).unwrap();
    let animal = ANIMALS.choose(&mut rng).unwrap();
    let random_number = rng.gen_range(0..100);

    format!("{}-{}-{}", adjective, animal, random_number)
}

pub(super) fn generate_run_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[allow(dead_code)]
pub(super) fn generate_pipeline_name_from_key(api_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(api_key);
    let hash = hasher.finalize();

    // Convert hash to a deterministic sequence
    let hash_bytes = hash.as_slice();

    // Use the hash to pick an adjective and an animal
    let animal_index = (hash_bytes[1] as usize) % ANIMALS.len();

    let animal = ANIMALS[animal_index];

    // Use part of the hash for a unique fragment
    let key_fragment = format!("{}", hash_bytes[hash_bytes.len() - 1]);

    format!("pipeline-{}-{}", animal, key_fragment)
}

#[cfg(test)]
mod tests {
    use super::generate_pipeline_name_from_key;

    #[test]
    fn test_pipeline_key_generates_same_name_per_key() {
        let api_key = "z12s33";
        let mut count = 0;
        let output = "pipeline-bear-247";

        while count < 5 {
            assert_eq!(output, generate_pipeline_name_from_key(api_key));
            count += 1
        }
    }
}
