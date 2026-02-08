//! Benchmarks for buffer performance
//!
//! Acceptance criteria: <10ms for view queries

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use gbe_buffer::{RingBuffer, RopeBuffer, ViewWindow};

fn bench_rope_insert(c: &mut Criterion) {
    c.bench_function("rope_insert_1000_chars", |b| {
        b.iter(|| {
            let mut buf = RopeBuffer::new();
            for i in 0..1000 {
                buf.insert(i, "x").unwrap();
            }
            black_box(buf);
        });
    });
}

fn bench_rope_view(c: &mut Criterion) {
    let mut group = c.benchmark_group("rope_view");

    for size in [100, 1000, 10000] {
        let mut buf = RopeBuffer::new();
        for i in 0..size {
            buf.insert(buf.len(), &format!("line {}\n", i)).unwrap();
        }

        group.bench_with_input(BenchmarkId::from_parameter(size), &buf, |b, buf| {
            b.iter(|| {
                let view = buf.view(ViewWindow::new(0, 100));
                black_box(view);
            });
        });
    }

    group.finish();
}

fn bench_rope_large_file(c: &mut Criterion) {
    c.bench_function("rope_view_large_file", |b| {
        // Simulate a 1MB file with 10K lines
        let mut content = String::new();
        for i in 0..10000 {
            content.push_str(&format!(
                "This is line {} with some content to simulate real text\n",
                i
            ));
        }
        let buf = RopeBuffer::with_content(&content);

        b.iter(|| {
            let view = buf.view(ViewWindow::new(5000, 100));
            black_box(view);
        });
    });
}

fn bench_ring_push(c: &mut Criterion) {
    c.bench_function("ring_push_1000_lines", |b| {
        b.iter(|| {
            let mut buf = RingBuffer::new(1000);
            for i in 0..1000 {
                buf.push(format!("line {}", i));
            }
            black_box(buf);
        });
    });
}

fn bench_ring_push_eviction(c: &mut Criterion) {
    c.bench_function("ring_push_with_eviction", |b| {
        b.iter(|| {
            let mut buf = RingBuffer::new(100);
            // Push 1000 lines to a buffer with capacity 100 (triggers eviction)
            for i in 0..1000 {
                buf.push(format!("line {}", i));
            }
            black_box(buf);
        });
    });
}

fn bench_ring_view(c: &mut Criterion) {
    let mut group = c.benchmark_group("ring_view");

    for capacity in [100, 1000, 10000] {
        let mut buf = RingBuffer::new(capacity);
        for i in 0..capacity {
            buf.push(format!("line {}", i));
        }

        group.bench_with_input(BenchmarkId::from_parameter(capacity), &buf, |b, buf| {
            b.iter(|| {
                let view = buf.view(ViewWindow::new(0, 100));
                black_box(view);
            });
        });
    }

    group.finish();
}

fn bench_ring_tail(c: &mut Criterion) {
    let mut buf = RingBuffer::new(10000);
    for i in 0..10000 {
        buf.push(format!("line {}", i));
    }

    c.bench_function("ring_tail_100", |b| {
        b.iter(|| {
            let tail = buf.tail(100);
            black_box(tail);
        });
    });
}

fn bench_ring_search(c: &mut Criterion) {
    let mut buf = RingBuffer::new(10000);
    for i in 0..10000 {
        if i % 100 == 0 {
            buf.push(format!("ERROR: line {}", i));
        } else {
            buf.push(format!("INFO: line {}", i));
        }
    }

    c.bench_function("ring_search", |b| {
        b.iter(|| {
            let results = buf.search("ERROR");
            black_box(results);
        });
    });
}

criterion_group!(
    rope_benches,
    bench_rope_insert,
    bench_rope_view,
    bench_rope_large_file
);

criterion_group!(
    ring_benches,
    bench_ring_push,
    bench_ring_push_eviction,
    bench_ring_view,
    bench_ring_tail,
    bench_ring_search
);

criterion_main!(rope_benches, ring_benches);
