#![forbid(unsafe_code)]
fn quicksort(xs: &mut [i64], mut lo: i64, mut hi: i64) {
    while lo < hi {
        let pivot = xs[((lo + hi) / 2) as usize];
        let mut i = lo;
        let mut j = hi;
        while i <= j {
            while xs[i as usize] < pivot {
                i += 1;
            }
            while xs[j as usize] > pivot {
                j -= 1;
            }
            if i <= j {
                let t = xs[i as usize];
                xs[i as usize] = xs[j as usize];
                xs[j as usize] = t;
                i += 1;
                j -= 1;
            }
        }
        if j - lo < hi - i {
            quicksort(xs, lo, j);
            lo = i;
        } else {
            quicksort(xs, i, hi);
            hi = j;
        }
    }
}
fn generate(count: usize) -> Vec<i64> {
    let mut xs = Vec::new();
    let mut seed = 12345_i64;
    for _ in 0..count {
        seed = (seed * 1103515245 + 12345) % 2147483648;
        xs.push(seed % 1000000);
    }
    xs
}
fn main() {
    let mut xs = generate(300000);
    let hi = xs.len() as i64 - 1;
    quicksort(&mut xs, 0, hi);
    let mut ok = true;
    for k in 1..xs.len() {
        if xs[k - 1] > xs[k] {
            ok = false;
        }
    }
    println!(
        "{} {} {} {}",
        if ok { "True" } else { "False" },
        xs[0],
        xs[xs.len() / 2],
        xs[xs.len() - 1]
    );
}
#[test]
fn boundary_vectors() {
    for mut xs in [
        vec![],
        vec![1],
        vec![2, 1],
        vec![1, 1, 1],
        vec![-3, 0, -1, 7],
        vec![5, 4, 3, 2, 1],
        vec![1, 2, 3, 4, 5],
        generate(300000),
    ] {
        let mut expected = xs.clone();
        expected.sort();
        let hi = xs.len() as i64 - 1;
        quicksort(&mut xs, 0, hi);
        assert_eq!(xs, expected);
    }
    assert_eq!(generate(5), vec![932606, 583775, 466924, 283573, 335178]);
}
