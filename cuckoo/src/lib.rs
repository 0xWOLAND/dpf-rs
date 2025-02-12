use hmac::{Hmac, Mac};
use rand::prelude::*;
use sha2::Sha256;
use thiserror::Error;

const MAX_EVICTIONS: usize = 500;
type HmacSha256 = Hmac<Sha256>;
const RANDOM_SEED: u64 = 12345;

pub fn prf(key: &[u8], seq_no: u64) -> Result<usize, Error> {
    let mut mac = HmacSha256::new_from_slice(key)
        .map_err(|e| Error::HmacError(e.to_string()))?;
    mac.update(&seq_no.to_be_bytes());
    let result = mac.finalize();
    let hash = result.into_bytes();
    Ok(usize::from_be_bytes(hash[0..8].try_into().unwrap()))
}


#[derive(Debug, Clone)]
pub struct Item {
    pub id: u64,
    pub data: Vec<u8>,
    pub seq_no: u64,
    pub bucket1: usize,
    pub bucket2: usize,
}

#[derive(Debug, Clone, Default)]
pub struct ItemLocation {
    id: u64,
    filled: bool,
    bucket1: usize,
    bucket2: usize,
    seq_no: u64,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid input: bucket indices must be less than num_buckets and item size must match")]
    InvalidInput,
    #[error("No space available after eviction")]
    NoSpaceAfterEviction,
    #[error("HMAC error: {0}")]
    HmacError(String),
}

pub struct Table {
    pub num_buckets: usize,
    pub bucket_depth: usize,
    pub item_size: usize,
    pub data: Vec<u8>,
    pub rng: StdRng,
    pub index: Vec<ItemLocation>,
    pub key1: Vec<u8>,
    pub key2: Vec<u8>,
}

impl Table {
    pub fn new(
        num_buckets: usize,
        bucket_depth: usize,
        item_size: usize,
        data: Option<Vec<u8>>,
        rand_seed: u64,
        key1: Vec<u8>,
        key2: Vec<u8>,
    ) -> Option<Self> {
        let expected_size = num_buckets * bucket_depth * item_size;
        let data = match data {
            Some(d) if d.len() != expected_size => return None,
            Some(d) => d,
            None => vec![0; expected_size],
        };

        Some(Self {
            num_buckets,
            bucket_depth,
            item_size,
            data,
            rng: StdRng::seed_from_u64(rand_seed),
            index: vec![ItemLocation::default(); num_buckets * bucket_depth],
            key1,
            key2,
        })
    }

    pub fn insert(&mut self, item: &Item) -> Result<Option<Item>, Error> {
        if item.data.len() != self.item_size {
            return Err(Error::InvalidInput);
        }

        let bucket1 = prf(&self.key1, item.seq_no)? % self.num_buckets;
        let bucket2 = prf(&self.key2, item.seq_no)? % self.num_buckets;

        if bucket1 != item.bucket1 || bucket2 != item.bucket2 {
            return Err(Error::InvalidInput);
        }

        let (first_bucket, other_bucket) = if self.rng.gen_bool(0.5) {
            (bucket1, bucket2)
        } else {
            (bucket2, bucket1)
        };

        if self.try_insert_to_bucket(first_bucket, item) {
            return Ok(None);
        }

        let mut next_bucket = other_bucket;
        let mut current_item = item.clone();

        for _ in 0..MAX_EVICTIONS {
            match self.insert_and_evict(next_bucket, &current_item)? {
                (true, None) => return Ok(None),
                (true, Some(evicted)) => {
                    current_item = evicted;
                    next_bucket = if current_item.bucket1 == next_bucket {
                        current_item.bucket2
                    } else {
                        current_item.bucket1
                    };
                }
                (false, Some(item)) => return Ok(Some(item)),
                _ => unreachable!(),
            }
        }
        Ok(Some(current_item))
    }

    pub fn get(&self, prf1: usize, prf2: usize) -> Option<Item> {
        // Ensure the two bucket indices are different.
        assert!(prf1 != prf2, "the two bucket indices cannot be the same");

        // Closure to search a given bucket for an item with the provided prf1 and prf2.
        let search_bucket = |bucket: usize| -> Option<Item> {
            let start_index = bucket * self.bucket_depth;
            let end_index = start_index + self.bucket_depth;
            for i in start_index..end_index {
                let slot = &self.index[i];
                if slot.filled && slot.bucket1 == prf1 && slot.bucket2 == prf2 {
                    return self.get_item(i);
                }
            }
            None
        };

        search_bucket(prf1).or_else(|| search_bucket(prf2))
    }

    fn try_insert_to_bucket(&mut self, bucket_index: usize, item: &Item) -> bool {
        let start = bucket_index * self.bucket_depth;
        let end = (bucket_index + 1) * self.bucket_depth;

        for i in start..end {
            if !self.index[i].filled {
                let data_start = i * self.item_size;
                self.data[data_start..data_start + item.data.len()].copy_from_slice(&item.data);
                self.index[i] = ItemLocation {
                    id: item.id,
                    filled: true,
                    bucket1: item.bucket1,
                    bucket2: item.bucket2,
                    seq_no: item.seq_no,
                };
                return true;
            }
        }
        false
    }

    fn insert_and_evict(
        &mut self,
        bucket_index: usize,
        item: &Item,
    ) -> Result<(bool, Option<Item>), Error> {
        if item.bucket1 != bucket_index && item.bucket2 != bucket_index {
            return Ok((false, Some(item.clone())));
        }

        if self.try_insert_to_bucket(bucket_index, item) {
            return Ok((true, None));
        }

        let evict_idx = bucket_index * self.bucket_depth + self.rng.gen_range(0..self.bucket_depth);
        let evicted_item = self.get_item(evict_idx).unwrap();
        self.index[evict_idx].filled = false;

        if !self.try_insert_to_bucket(bucket_index, item) {
            return Err(Error::NoSpaceAfterEviction);
        }

        Ok((true, Some(evicted_item)))
    }

    fn get_item(&self, item_index: usize) -> Option<Item> {
        if !self.index[item_index].filled {
            return None;
        }
        let data_start = item_index * self.item_size;
        Some(Item {
            id: self.index[item_index].id,
            data: self.data[data_start..data_start + self.item_size].to_vec(),
            bucket1: self.index[item_index].bucket1,
            bucket2: self.index[item_index].bucket2,
            seq_no: self.index[item_index].seq_no,
        })
    }
}

impl Item {
    pub fn new(id: u64, data: Vec<u8>, seq_no: u64, bucket1: usize, bucket2: usize) -> Self {
        Self {
            id,
            data,
            seq_no,
            bucket1,
            bucket2,
        }
    }
}

impl PartialEq for Item {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.bucket1 == other.bucket1 && self.bucket2 == other.bucket2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::prelude::*;

    const TEST_ITEM_SIZE: usize = 64;
    const TEST_KEY1: &[u8] = b"test_key_1_for_prf_computation";
    const TEST_KEY2: &[u8] = b"test_key_2_for_prf_computation";

    fn get_bytes(val: &str) -> Vec<u8> {
        let mut buf = vec![0; TEST_ITEM_SIZE];
        buf[..val.len()].copy_from_slice(val.as_bytes());
        buf
    }

    fn create_test_table(num_buckets: usize, bucket_depth: usize) -> Table {
        Table::new(
            num_buckets,
            bucket_depth,
            TEST_ITEM_SIZE,
            None,
            RANDOM_SEED,
            TEST_KEY1.to_vec(),
            TEST_KEY2.to_vec(),
        )
        .unwrap()
    }

    fn create_test_item(table: &Table, id: u64, data: Vec<u8>, seq_no: u64) -> Item {
        let bucket1 = prf(TEST_KEY1, seq_no).unwrap() % table.num_buckets;
        let bucket2 = prf(TEST_KEY2, seq_no).unwrap() % table.num_buckets;
        Item::new(id, data, seq_no, bucket1, bucket2)
    }

    #[test]
    fn test_get_capacity() {
        let table = create_test_table(10, 2);
        assert_eq!(10 * 2, table.index.len());

        let table = create_test_table(1, 1);
        assert_eq!(1, table.index.len());

        let table = create_test_table(0, 0);
        assert_eq!(0, table.index.len());
    }

    #[test]
    fn test_invalid_construction() {
        let data = vec![0; 7];
        let table = Table::new(2, 3, 4, Some(data), 0, TEST_KEY1.to_vec(), TEST_KEY2.to_vec());
        assert!(table.is_none());
    }

    #[test]
    fn test_basic() {
        let mut table = create_test_table(10, 2);
        assert_eq!(0, table.index.iter().filter(|loc| loc.filled).count());

        // Test with invalid data size
        let item = create_test_item(&table, 1, vec![0, 0], 0);
        let result = table.insert(&item);
        assert!(result.is_err());

        // Test with valid data
        let item = create_test_item(&table, 1, get_bytes("value1"), 0);
        let result = table.insert(&item);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        assert!(table.index.iter().any(|loc| loc.filled && loc.id == 1));
        assert_eq!(1, table.index.iter().filter(|loc| loc.filled).count());
    }

    #[test]
    fn test_out_of_bounds() {
        let mut table = create_test_table(10, 2);
        
        // Create item with invalid buckets (not matching PRF)
        let item = Item::new(1, get_bytes("value1"), 0, 100, 100);
        let result = table.insert(&item);
        assert!(result.is_err());
    }

    #[test]
    fn test_full_table() {
        let num_buckets = 100;
        let depth = 4;
        let capacity = num_buckets * depth;
        let mut table = create_test_table(num_buckets, depth);
        let mut rng = StdRng::seed_from_u64(RANDOM_SEED);

        let mut count = 0;
        let mut entries = Vec::with_capacity(capacity);
        let mut evicted = None;
        let mut seq_no = 0u64;

        loop {
            let id = rng.gen::<u64>();
            let val = get_bytes(&rng.gen::<u64>().to_string());
            
            let item = create_test_item(&table, id, val, seq_no);
            let empty_item = create_test_item(&table, id, vec![], seq_no);
            entries.push(empty_item);
            seq_no += 1;

            match table.insert(&item) {
                Ok(None) => {
                    count += 1;
                    let found = table.index.iter().any(|loc| {
                        loc.filled && loc.id == id && loc.bucket1 == item.bucket1 && loc.bucket2 == item.bucket2
                    });
                    assert!(found, "Insert() succeeded, but item not found in table");

                    let actual_count = table.index.iter().filter(|loc| loc.filled).count();
                    assert_eq!(
                        count, actual_count,
                        "Number of successful inserts ({}) does not match actual elements ({})",
                        count, actual_count
                    );
                }
                Ok(Some(e)) => {
                    evicted = Some(e);
                    break;
                }
                Err(_) => break,
            }
        }

        let actual_count = table.index.iter().filter(|loc| loc.filled).count();
        assert_eq!(
            count, actual_count,
            "Number of successful inserts ({}) does not match actual elements ({})",
            count, actual_count
        );
        let max_count = count;

        for entry in entries {
            if Some(&entry) != evicted.as_ref() {
                let found = table.index.iter().any(|loc| {
                    loc.filled
                        && loc.bucket1 == entry.bucket1
                        && loc.bucket2 == entry.bucket2
                        && loc.id == entry.id
                });
                assert!(
                    found,
                    "Cannot find element believed to be in table. item {} of {}",
                    count, max_count
                );

                for loc in table.index.iter_mut() {
                    if loc.filled && loc.id == entry.id {
                        loc.filled = false;
                        count -= 1;
                        break;
                    }
                }

                let actual_count = table.index.iter().filter(|loc| loc.filled).count();
                assert_eq!(
                    count, actual_count,
                    "GetNumElements()={} returned value that didn't match expected={}",
                    actual_count, count
                );
            }
        }

        let final_count = table.index.iter().filter(|loc| loc.filled).count();
        assert_eq!(
            0, final_count,
            "GetNumElements() returns {} when table should be empty",
            final_count
        );
    }

    #[test]
    fn test_duplicate_values() {
        let mut table = create_test_table(10, 2);

        let item1 = create_test_item(&table, 1, get_bytes("v"), 0);
        let result = table.insert(&item1);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        let item2 = create_test_item(&table, 2, get_bytes("v"), 1);
        let result = table.insert(&item2);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        let item3 = create_test_item(&table, 3, get_bytes("v"), 2);
        let result = table.insert(&item3);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_prf_consistency() {
        let table = create_test_table(10, 2);
        
        // Same seq_no should produce same buckets
        let bucket1_a = prf(TEST_KEY1, 0).unwrap() % table.num_buckets;
        let bucket1_b = prf(TEST_KEY1, 0).unwrap() % table.num_buckets;
        assert_eq!(bucket1_a, bucket1_b);

        let bucket2_a = prf(TEST_KEY2, 0).unwrap() % table.num_buckets;
        let bucket2_b = prf(TEST_KEY2, 0).unwrap() % table.num_buckets;
        assert_eq!(bucket2_a, bucket2_b);

        // Different seq_no should produce different buckets
        let bucket1_c = prf(TEST_KEY1, 1).unwrap() % table.num_buckets;
        assert_ne!(bucket1_a, bucket1_c);
    }

    #[test]
    fn test_get_item() {
        let mut table = create_test_table(10, 2);
        let seq_no = 42;
        let id = 100;
        let data = get_bytes("test_value");
        let item = create_test_item(&table, id, data, seq_no);

        // Insert the item.
        let result = table.insert(&item);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        // Now, retrieve the item using its two distinct PRF bucket indices.
        let retrieved = table.get(item.bucket1, item.bucket2);
        assert!(retrieved.is_some(), "Expected to retrieve the inserted item");
        let retrieved_item = retrieved.unwrap();
        assert_eq!(retrieved_item.id, item.id, "The retrieved item id does not match");
        assert_eq!(retrieved_item.data, item.data, "The retrieved item data does not match");
        assert_eq!(retrieved_item.bucket1, item.bucket1, "Bucket1 does not match");
        assert_eq!(retrieved_item.bucket2, item.bucket2, "Bucket2 does not match");
    }
}
