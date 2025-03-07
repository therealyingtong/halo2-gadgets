use std::array;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use ff::Field;
use sinsemilla::primitive;

use pasta_curves::pallas;
#[cfg(unix)]
use pprof::criterion::{Output, PProfProfiler};
use rand::{rngs::OsRng, Rng};

fn bench_primitives(c: &mut Criterion) {
    let mut rng = OsRng;

    {
        let mut group = c.benchmark_group("Primitiprimitive");

        let hasher = primitive::HashDomain::new("hasher");
        let committer = primitive::CommitDomain::new("committer");
        let bits: Vec<bool> = (0..1086).map(|_| rng.gen()).collect();
        let r = pallas::Scalar::random(rng);

        // Benchmark the input sizes we use in Orchard:
        // - 510 bits for Commit^ivk
        // - 520 bits for MerkleCRH
        // - 1086 bits for NoteCommit
        for size in array::IntoIter::new([510, 520, 1086]) {
            group.bench_function(BenchmarkId::new("hash-to-point", size), |b| {
                b.iter(|| hasher.hash_to_point(bits[..size].iter().cloned()))
            });

            group.bench_function(BenchmarkId::new("hash", size), |b| {
                b.iter(|| hasher.hash(bits[..size].iter().cloned()))
            });

            group.bench_function(BenchmarkId::new("commit", size), |b| {
                b.iter(|| committer.commit(bits[..size].iter().cloned(), &r))
            });

            group.bench_function(BenchmarkId::new("short-commit", size), |b| {
                b.iter(|| committer.commit(bits[..size].iter().cloned(), &r))
            });
        }
    }
}

#[cfg(unix)]
criterion_group! {
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = bench_primitives
}
#[cfg(not(unix))]
criterion_group!(benches, bench_primitives);
criterion_main!(benches);
