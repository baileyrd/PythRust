//! Python slice normalization over code-point or element positions.
//! Wider arithmetic is internal: even the final unused increment is safe.

#[derive(Clone, Debug)]
pub struct Indices {
    next: i128,
    stop: i128,
    step: i128,
}

pub fn indices(len: usize, start: Option<i64>, stop: Option<i64>, step: i64) -> Option<Indices> {
    if step == 0 {
        return None;
    }
    let n = len as i128;
    let backwards = step < 0;
    let low = if backwards { -1 } else { 0 };
    let high = if backwards { n - 1 } else { n };
    let bound = |v: i64| {
        let v = i128::from(v);
        let v = if v < 0 { v + n } else { v };
        v.clamp(low, high)
    };
    Some(Indices {
        next: start
            .map(bound)
            .unwrap_or(if backwards { n - 1 } else { 0 }),
        stop: stop.map(bound).unwrap_or(if backwards { -1 } else { n }),
        step: i128::from(step),
    })
}

impl Iterator for Indices {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        let active = if self.step < 0 {
            self.next > self.stop
        } else {
            self.next < self.stop
        };
        if !active {
            return None;
        }
        let at = self.next as usize; // normalized into [0, len)
        self.next += self.step;
        Some(at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omitted_stop_and_extremes() {
        let collect = |n, a, b, c| indices(n, a, b, c).unwrap().collect::<Vec<_>>();
        assert_eq!(collect(5, None, None, -1), [4, 3, 2, 1, 0]);
        assert_eq!(collect(5, None, Some(-1), -1), []);
        assert_eq!(collect(0, None, None, i64::MIN), []);
        assert_eq!(
            collect(5, Some(i64::MIN), Some(i64::MAX), 1),
            [0, 1, 2, 3, 4]
        );
        assert_eq!(collect(5, None, None, i64::MIN), [4]);
        assert_eq!(collect(5, None, None, i64::MAX), [0]);
        assert!(indices(5, None, None, 0).is_none());
    }
}
