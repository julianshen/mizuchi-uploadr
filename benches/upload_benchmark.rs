//! Upload benchmarks

use bytes::Bytes;
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

fn benchmark_upload_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("upload_sizes");

    for size in [1024, 10 * 1024, 100 * 1024, 1024 * 1024].iter() {
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(format!("{}_bytes", size), size, |b, &size| {
            let data = Bytes::from(vec![0u8; size]);
            b.iter(|| {
                black_box(&data);
            });
        });
    }

    group.finish();
}

criterion_group!(benches, benchmark_upload_sizes);
criterion_main!(benches);
