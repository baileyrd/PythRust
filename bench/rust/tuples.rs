#![forbid(unsafe_code)]
fn div_mod(a: i64, b: i64) -> (i64, i64) {
    (a / b, a % b)
}
fn digits(mut n: i64) -> (i64, i64) {
    let mut count = 0;
    let mut total = 0;
    while n > 0 {
        let (next, d) = div_mod(n, 10);
        n = next;
        count += 1;
        total += d;
    }
    (count, total)
}
fn main() {
    let mut all_count = 0_i64;
    let mut all_total = 0_i64;
    for i in 0..1000000 {
        let (c, t) = digits(i);
        all_count += c;
        all_total += t;
    }
    println!("{all_count} {all_total}");
}
#[test]
fn digit_boundaries() {
    for (n, c, t) in [
        (0, 0, 0),
        (1, 1, 1),
        (9, 1, 9),
        (10, 2, 1),
        (99, 2, 18),
        (100, 3, 1),
        (999999, 6, 54),
    ] {
        assert_eq!(digits(n), (c, t));
    }
    assert_eq!(div_mod(99, 10), (9, 9));
}
