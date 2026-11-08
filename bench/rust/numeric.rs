#![forbid(unsafe_code)]
fn steps(mut n: i64) -> i64 {
    let mut count = 0;
    while n != 1 {
        if n % 2 == 0 {
            n /= 2;
        } else {
            n = 3 * n + 1;
        }
        count += 1;
    }
    count
}
fn main() {
    let mut total = 0_i64;
    let mut longest = 0_i64;
    for i in 1..1_000_000 {
        let s = steps(i);
        total += s;
        if s > longest {
            longest = s;
        }
    }
    println!("{total} {longest}");
}
#[test]
fn known_steps() {
    for (n, expected) in [(1, 0), (2, 1), (3, 7), (27, 111)] {
        assert_eq!(steps(n), expected);
    }
}
