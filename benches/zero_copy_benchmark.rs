//! Zero-copy transfer benchmarks

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

fn benchmark_zero_copy_available(c: &mut Criterion) {
    c.bench_function("zero_copy_available_check", |b| {
        b.iter(|| {
            black_box(mizuchi_uploadr::zero_copy_available());
        });
    });
}

fn benchmark_data_transfer_creation(c: &mut Criterion) {
    use mizuchi_uploadr::upload::zero_copy::{DataTransfer, DEFAULT_BUFFER_SIZE};
    
    c.bench_function("data_transfer_creation", |b| {
        b.iter(|| {
            let transfer = DataTransfer::new(DEFAULT_BUFFER_SIZE, true);
            black_box(transfer);
        });
    });
}

criterion_group!(benches, benchmark_zero_copy_available, benchmark_data_transfer_creation);
criterion_main!(benches);
