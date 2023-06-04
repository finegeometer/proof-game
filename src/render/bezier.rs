pub fn path(
    start: [f64; 2],
    start_vector: [f64; 2],
    end_vector: [f64; 2],
    end: [f64; 2],
    d: &mut dodrio::bumpalo::collections::String,
) {
    use std::fmt::Write;
    write!(
        d,
        "M {} {} C {} {}, {} {}, {} {}",
        start[0],
        start[1],
        start[0] + start_vector[0],
        start[1] + start_vector[1],
        end[0] - end_vector[0],
        end[1] - end_vector[1],
        end[0],
        end[1],
    )
    .unwrap();
}

pub fn split(
    start: [f64; 2],
    start_vector: [f64; 2],
    end_vector: [f64; 2],
    end: [f64; 2],
) -> ([f64; 2], [f64; 2]) {
    (
        [
            (3. * start_vector[0] + 2. * start[0] - 3. * end_vector[0] + 2. * end[0]) / 4.,
            (3. * start_vector[1] + 2. * start[1] - 3. * end_vector[1] + 2. * end[1]) / 4.,
        ],
        [
            (end[0] - end_vector[0] - start_vector[0] - start[0]) / 4.,
            (end[1] - end_vector[1] - start_vector[1] - start[1]) / 4.,
        ],
    )
}

pub fn average(vs: &[[f64; 2]]) -> [f64; 2] {
    let mut x = 0.;
    let mut y = 0.;
    for v in vs {
        x += v[0];
        y += v[1];
    }
    let n = vs.len() as f64;
    [x / n, y / n]
}
