use std::{cmp::Ordering, collections::BinaryHeap};

use nonempty::NonEmpty;
use url::Url;

/// A wrapper around a priority queue of URLs that handles penalization and weighting.
#[derive(Debug)]
pub struct UrlPool {
    heap: BinaryHeap<WeightedUrl>,
}

impl UrlPool {
    /// Creates a new pool from a non-empty list of URLs.
    pub fn new(urls: NonEmpty<Url>) -> Self {
        let mut heap = BinaryHeap::with_capacity(urls.len());
        for url in urls {
            heap.push(WeightedUrl::from(url));
        }
        Self { heap }
    }

    /// Pulls the highest priority URL from the pool.
    pub fn pop(&mut self) -> Option<WeightedUrl> {
        self.heap.pop()
    }

    /// Re-inserts a successful URL with increased priority and penalizes failed ones.
    pub fn report_success(&mut self, mut winner: WeightedUrl, failures: Vec<WeightedUrl>) {
        winner.weight += 1;
        self.heap.push(winner);
        for mut loser in failures {
            loser.weight -= 10;
            self.heap.push(loser);
        }
    }

    /// Re-inserts all URLs with a penalty when the entire operation failed.
    pub fn report_failures(&mut self, failures: Vec<WeightedUrl>) {
        for mut loser in failures {
            loser.weight -= 10;
            self.heap.push(loser);
        }
    }
}

#[derive(Debug, Clone)]
pub struct WeightedUrl {
    pub url: Url,
    pub weight: i32,
}

impl PartialEq for WeightedUrl {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url
    }
}
impl Eq for WeightedUrl {}
impl PartialOrd for WeightedUrl {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for WeightedUrl {
    fn cmp(&self, other: &Self) -> Ordering {
        self.weight.cmp(&other.weight)
    }
}
impl From<Url> for WeightedUrl {
    fn from(url: Url) -> Self {
        WeightedUrl { url, weight: 0 }
    }
}
