use std::collections::HashSet;

use pairing::{CurveAffine, CurveProjective,};

pub struct BucketAdder<G: CurveAffine> {
    buckets: Vec<G>,
    a: Vec<G>,
    b: Vec<G>,
    indexes: Vec<usize>,
    count: usize,
    busy: HashSet<usize>,
    collisions: Vec<(G, usize)>,
    buffer_size: usize,
}

impl<G: CurveAffine> BucketAdder<G> {
    pub fn new(c: u32, chunk: usize) -> BucketAdder<G> {
        let zero = G::zero();
        let buffer_size = 256;
        BucketAdder { 
            buckets: vec![zero; (1 << c) - 1], 
            a: vec![zero; buffer_size], 
            b: vec![zero; buffer_size], 
            indexes: vec![0; buffer_size],
            count: 0,
            busy: HashSet::new(),
            collisions: Vec::new(),
            buffer_size,
        }
    }

    fn add(&mut self, base: G, bucket: usize) {
        // perform "addition" in place
        if self.buckets[bucket] == G::zero() {
            self.buckets[bucket] = base;
            return
        } else if base == G::zero() {
            return
        }
        
        // bucket is already in buffer, postpone that addition
        if self.busy.contains(&bucket) {
            self.collisions.push((base, bucket));
            return;
        }

        // batch addtition doesn't work with equal points
        // so we need to perform this addition in place
        // even though it is very slow because of into_affine
        if self.buckets[bucket] == base {
            let mut p = self.buckets[bucket].into_projective();
            p.add_assign_mixed(&base);
            self.buckets[bucket] = p.into_affine();
            return;
        }

        self.a[self.count] = self.buckets[bucket];
        self.b[self.count] = base;
        self.indexes[self.count] = bucket;
        self.busy.insert(bucket);
        self.count += 1;
    }

    pub fn add_to_bucket(&mut self, base: G, bucket: usize) {
        self.add(base, bucket);

        if self.count == self.buffer_size || self.collisions.len() >= self.buffer_size / 2  {
            self.process_buffer();
        }
    }

    pub fn finalize(mut self) -> Vec<G> {
        while !self.collisions.is_empty() {
            self.process_buffer();
        }
        self.process_buffer();
        self.buckets
    }
    
    fn process_buffer(&mut self) {
        CurveAffine::batch_addition_assign(&mut self.a[0..self.count], &self.b[0..self.count]);

        for i in 0..self.count {
            self.buckets[self.indexes[i]] = self.a[i];
        }

        self.count = 0;
        self.busy.clear();

        let collisions = self.collisions.clone();
        self.collisions.clear();
        for (base, bucket) in collisions {
            self.add(base, bucket)
        }
    }
}