use std::collections::HashSet;

use pairing::{CurveAffine, CurveProjective,};

const BATCH_SIZE: usize = 256;

pub struct BucketAdder<G: CurveAffine> {
    buckets: Vec<G>,
    
    batch_lhs: Vec<G>,
    batch_rhs: Vec<G>,
    batch_indexes: Vec<usize>,
    batch_count: usize,
    buckets_in_batch: HashSet<usize>,
    collisions: Vec<(G, usize)>,
    batch_size: usize,
}

impl<G: CurveAffine> BucketAdder<G> {
    pub fn new(c: u32, _chunk: usize) -> BucketAdder<G> {
        let zero = G::zero();
        let batch_size = BATCH_SIZE;
        BucketAdder { 
            buckets: vec![zero; (1 << c) - 1], 
            batch_lhs: vec![zero; batch_size], 
            batch_rhs: vec![zero; batch_size], 
            batch_indexes: vec![0; batch_size],
            batch_count: 0,
            buckets_in_batch: HashSet::new(),
            collisions: Vec::new(),
            batch_size,
        }
    }

    pub fn add_to_bucket(&mut self, base: G, bucket: usize) {
        self.add(base, bucket);

        if self.batch_count == self.batch_size || self.collisions.len() >= self.batch_size / 2  {
            self.process_batch();
        }
    }

    pub fn finalize(mut self) -> Vec<G> {
        while !self.collisions.is_empty() {
            self.process_batch();
        }
        self.process_batch();
        self.buckets
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
        if self.buckets_in_batch.contains(&bucket) {
            self.collisions.push((base, bucket));
            return;
        }

        // batch addtition doesn't work if P = +-Q
        // so we need to perform this addition in place
        // even though it is slow because of into_affine
        if self.buckets[bucket].get_x() == base.get_x() {
            let mut p = self.buckets[bucket].into_projective();
            p.add_assign_mixed(&base);
            self.buckets[bucket] = p.into_affine();
            return;
        }

        self.batch_lhs[self.batch_count] = self.buckets[bucket];
        self.batch_rhs[self.batch_count] = base;
        self.batch_indexes[self.batch_count] = bucket;
        self.buckets_in_batch.insert(bucket);
        self.batch_count += 1;
    }
    
    fn process_batch(&mut self) {
        CurveAffine::batch_addition_assign(&mut self.batch_lhs[0..self.batch_count], &self.batch_rhs[0..self.batch_count]);

        for i in 0..self.batch_count {
            self.buckets[self.batch_indexes[i]] = self.batch_lhs[i];
        }

        self.batch_count = 0;
        self.buckets_in_batch.clear();

        let collisions = self.collisions.clone();
        self.collisions.clear();
        for (base, bucket) in collisions {
            self.add(base, bucket)
        }
    }
}