use criterion::{Criterion, black_box, criterion_group, criterion_main};
use gpui_ui_kit::input::edit_state::EditState;

/// Build a ~200-character string of space-separated words.
fn words_text() -> String {
    std::iter::repeat("word ").take(40).collect()
}

fn bench_insert_char(c: &mut Criterion) {
    let text = words_text();
    c.bench_function("insert_char", |b| {
        b.iter_with_setup(
            || {
                let mut state = EditState::new(black_box(&text));
                state.clear_selection();
                state.move_to_end();
                state
            },
            |mut state| {
                state.insert_char(black_box('x'));
                black_box(state);
            },
        );
    });
}

fn bench_backspace(c: &mut Criterion) {
    let text = words_text();
    c.bench_function("backspace", |b| {
        b.iter_with_setup(
            || {
                let mut state = EditState::new(black_box(&text));
                state.clear_selection();
                state.move_to_end();
                state
            },
            |mut state| {
                state.do_backspace();
                black_box(state);
            },
        );
    });
}

fn bench_kill_word_backward(c: &mut Criterion) {
    let text = words_text();
    c.bench_function("kill_word_backward", |b| {
        b.iter_with_setup(
            || EditState::new(black_box(&text)),
            |mut state| {
                state.kill_word_backward();
                black_box(state);
            },
        );
    });
}

fn bench_delete_selection(c: &mut Criterion) {
    let text = words_text();
    c.bench_function("delete_selection", |b| {
        b.iter_with_setup(
            || EditState::new(black_box(&text)),
            |mut state| {
                state.delete_selection();
                black_box(state);
            },
        );
    });
}

criterion_group!(
    benches,
    bench_insert_char,
    bench_backspace,
    bench_kill_word_backward,
    bench_delete_selection
);
criterion_main!(benches);
