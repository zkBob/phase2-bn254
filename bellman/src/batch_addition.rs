use pairing::{CurveAffine, CurveProjective,};

const BATCH_SIZE: usize = 1024;
const MAX_COLLISIONS_COUNT: usize = 128;

pub struct BucketAdder<G: CurveAffine> {
    buckets: Vec<G>,
    
    batch_lhs: Vec<G>,
    batch_rhs: Vec<G>,
    batch_buckets: Vec<usize>,
    batch_count: usize,
    
    bucket_in_batch: Vec<bool>,
    collisions: Vec<(G, usize)>,
    collisions_count: usize,

    scratch_space: Vec<G::Base>,
}

impl<G: CurveAffine> BucketAdder<G> {
    pub fn new(c: u32, _chunk: usize) -> BucketAdder<G> {
        let zero = G::zero();
        BucketAdder { 
            buckets: vec![zero; (1 << c) - 1], 
            batch_lhs: vec![zero; BATCH_SIZE], 
            batch_rhs: vec![zero; BATCH_SIZE], 
            scratch_space: vec![zero.get_x(); BATCH_SIZE],
            batch_buckets: vec![0; BATCH_SIZE],
            batch_count: 0,
            bucket_in_batch: vec![false; (1 << c) - 1],
            collisions: vec![(zero, 0); MAX_COLLISIONS_COUNT],
            collisions_count: 0,
        }
    }

    pub fn add_to_bucket(&mut self, base: G, bucket: usize) {
        self.add(base, bucket);

        if self.batch_count == BATCH_SIZE || self.collisions_count >= MAX_COLLISIONS_COUNT  {
            self.process_batch();
        }
    }

    pub fn finalize(mut self) -> Vec<G> {
        while self.collisions_count > 0 {
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
        
        // bucket is already in batch, postpone that addition
        if self.bucket_in_batch[bucket] {
            self.collisions[self.collisions_count] = (base, bucket);
            self.collisions_count += 1;
            return;
        }

        // batch addtition doesn't work if P = +-Q
        // so we need to perform this addition in place
        // even though it is slow
        if self.buckets[bucket].get_x() == base.get_x() {
            let mut p = self.buckets[bucket].into_projective();
            p.add_assign_mixed(&base);
            self.buckets[bucket] = p.into_affine();
            return;
        }

        self.batch_lhs[self.batch_count] = self.buckets[bucket];
        self.batch_rhs[self.batch_count] = base;
        self.batch_buckets[self.batch_count] = bucket;
        self.bucket_in_batch[bucket] = true;
        self.batch_count += 1;
    }
    
    fn process_batch(&mut self) {
        CurveAffine::batch_addition_assign(&mut self.batch_lhs[0..self.batch_count], &self.batch_rhs[0..self.batch_count], &mut self.scratch_space);

        for i in 0..self.batch_count {
            let bucket = self.batch_buckets[i];
            self.buckets[bucket] = self.batch_lhs[i];
            self.bucket_in_batch[bucket] = false;
        }

        self.batch_count = 0;

        let collisions_count = self.collisions_count;
        self.collisions_count = 0;
        for i in 0..collisions_count {
            let collision = self.collisions[i];
            self.add(collision.0, collision.1);
        }
    }
}