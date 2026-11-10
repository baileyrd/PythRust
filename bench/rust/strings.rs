#![forbid(unsafe_code)]
fn work(limit: i64) -> (i64, usize) {
    let mut total = 0_i64;
    let mut line = String::new();
    for i in 0..limit {
        let key = String::from("k") + &i.to_string() + ":" + &(i % 97).to_string();
        #[cfg(materialize)]
        std::hint::black_box(&key);
        total += key.len() as i64;
        if i % 50 == 0 {
            line += &key[0..1];
        }
    }
    #[cfg(materialize)]
    std::hint::black_box(&line);
    (total, line.len())
}
fn main() {
    let (total, len) = work(1000000);
    println!("{total} {len}");
}
#[test]
fn boundary_strings() {
    for n in [0_i64, 1, 49, 50, 51, 96, 97, 98, 101] {
        let expected: i64 = (0..n)
            .map(|i| format!("k{}:{}", i, i % 97).len() as i64)
            .sum();
        assert_eq!(work(n), (expected, ((n + 49) / 50) as usize));
    }
    assert_eq!(i64::MIN.to_string(), "-9223372036854775808");
    assert_eq!(i64::MAX.to_string(), "9223372036854775807");
    assert_eq!("é😀".chars().count(), 2);
}
