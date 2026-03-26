/// xoshiro256** PRNG - fast, high-quality, seedable
#[derive(Debug, Clone)]
pub struct Rng {
    state: [u64; 4],
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        // Use splitmix64 to initialize the state from a single seed
        let mut sm = SplitMix64(seed);
        let state = [sm.next(), sm.next(), sm.next(), sm.next()];
        Self { state }
    }

    pub fn from_seed_pair(seed1: u64, seed2: u64) -> Self {
        Self::new(seed1.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(seed2))
    }

    /// Returns a u64 in [0, u64::MAX]
    pub fn next_u64(&mut self) -> u64 {
        let result = (self.state[1].wrapping_mul(5)).rotate_left(7).wrapping_mul(9);
        let t = self.state[1] << 17;

        self.state[2] ^= self.state[0];
        self.state[3] ^= self.state[1];
        self.state[1] ^= self.state[2];
        self.state[0] ^= self.state[3];

        self.state[2] ^= t;
        self.state[3] = self.state[3].rotate_left(45);

        result
    }

    /// Returns a f64 in [0, 1)
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Returns a f64 in [min, max)
    pub fn range_f64(&mut self, min: f64, max: f64) -> f64 {
        min + self.next_f64() * (max - min)
    }

    /// Returns a u32 in [0, max)
    pub fn range_u32(&mut self, max: u32) -> u32 {
        (self.next_u64() % max as u64) as u32
    }

    /// Returns true with probability p
    pub fn chance(&mut self, p: f64) -> bool {
        self.next_f64() < p
    }

    /// Pick a random element from a slice
    pub fn choose<'a, T>(&mut self, slice: &'a [T]) -> &'a T {
        let idx = self.range_u32(slice.len() as u32) as usize;
        &slice[idx]
    }

    /// Shuffle a slice in place
    pub fn shuffle<T>(&mut self, slice: &mut [T]) {
        for i in (1..slice.len()).rev() {
            let j = self.range_u32((i + 1) as u32) as usize;
            slice.swap(i, j);
        }
    }

    /// Random angle in [0, 2π)
    pub fn angle(&mut self) -> f64 {
        self.next_f64() * std::f64::consts::TAU
    }
}

/// SplitMix64 used for seeding xoshiro
struct SplitMix64(u64);

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
}

/// Hash two u64 values together (for level seed computation)
pub fn hash_seeds(a: u64, b: u64) -> u64 {
    let mut sm = SplitMix64(a.wrapping_mul(0x517CC1B727220A95).wrapping_add(b));
    sm.next()
}
