use criterion::{black_box, Criterion, criterion_group, criterion_main, ParameterizedBenchmark};
use rand::prelude::*;

fn rand_string(rng: &mut impl Rng, len: usize) -> String {
    let v: Vec<u8> = (0..len).map(|_| rng.gen_range(0, 128)).collect();
    String::from_utf8(v).unwrap()
}

/// Expands a set of inputs to a fixed length vector and optionally replaces some percentage of
/// inputs with a random string.
fn make_input<'a>(atoms: impl Clone + Iterator<Item = &'a str>, len: usize, miss_rate: f64) -> Vec<String> {
    let mut rng = StdRng::seed_from_u64(42);

    let max_len = atoms.clone().map(|s| s.len()).max().unwrap_or(1);
    let misses = (len as f64 * miss_rate) as usize;

    let mut cycled: Vec<String> = atoms.cycle()
        .take(len)
        .map(ToOwned::to_owned)
        .collect();

    let replace = rand::seq::index::sample(&mut rng, len, misses);

    for i in replace.into_iter() {
        let len = rng.gen_range(1, max_len);
        cycled[i] = rand_string(&mut rng, len);
    }

    cycled.shuffle(&mut rng);
    cycled
}

// The extra macro is needed because the first fn has to be passed to `new`
macro_rules! parameterized {
    ($params:expr, $hn:ident, $head:expr, $( $tn:ident, $tail:expr ),* $(,)?) => {
        ParameterizedBenchmark::new(stringify!($hn), $head, $params)
            $( .with_function(stringify!($tn), $tail) )*
    }
}

macro_rules! benches {
    ($( $data:ident => [$($method:ident),*]; )*) => {
        $(
            pub mod $data {
                use super::*;

                $(
                    include!(concat!(env!("OUT_DIR"), "/", stringify!($data), "_", stringify!($method), ".rs"));
                )*

                pub fn run(c: &mut Criterion) {
                    static DATA: &str = include_str!(concat!("../data/", stringify!($data), ".txt"));

                    let miss_rates = vec![0.0, 0.25, 0.5, 0.75];
                    let b = parameterized!(
                        miss_rates,
                        $(
                            $method,
                            move |b, &miss_rate| {
                                let data = make_input(DATA.lines(), 5000, miss_rate);

                                b.iter(|| {
                                    for atom in &*data {
                                        black_box($method(atom));
                                    }
                                })
                            },
                        )*
                    );

                    c.bench(stringify!($data), b);
                }
            }
        )*

        criterion_group!(comparative, $( $data::run ),*);
        criterion_main!(comparative);
    }
}

benches! {
    english => [control, phf, trie, gperf];
    rust => [control, phf, trie, gperf];
    http => [control, phf, trie, gperf];
    google_1000_english_no_swears => [control, phf, trie, gperf];
}
