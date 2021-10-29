use std::collections::BTreeMap;

#[derive(Debug)]
pub struct Histogram<T> {
    inner: BTreeMap<T, usize>,
}

pub struct Stats<T> {
    pub total: usize,
    pub median: T,
    pub p90: T,
    pub p99: T,
    pub p999: T,
}

impl<T> Histogram<T>
where
    T: Ord + Copy,
{
    pub fn new() -> Self {
        Self {
            inner: BTreeMap::new(),
        }
    }

    pub fn record(&mut self, key: T) {
        let bucket = self.inner.entry(key).or_default();
        *bucket += 1;
    }

    pub fn stats(&self) -> Stats<T> {
        let total = self.inner.values().sum();

        let mut sum = 0;
        let mut median = None;
        let mut p90 = None;
        let mut p99 = None;
        let mut p999 = None;
        for (item, count) in &self.inner {
            sum += count;
            if median.is_none() && sum > total / 2 {
                median = Some(*item);
            }
            if p90.is_none() && sum > total * 90 / 100 {
                p90 = Some(*item);
            }
            if p99.is_none() && sum > total * 99 / 100 {
                p99 = Some(*item);
            }
            if p999.is_none() && sum > total * 999 / 1000 {
                p999 = Some(*item);
                break;
            }
        }
        Stats {
            total,
            median: median.unwrap(),
            p90: p90.unwrap(),
            p99: p99.unwrap(),
            p999: p999.unwrap(),
        }
    }
}
