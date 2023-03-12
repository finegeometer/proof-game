/// Given points `A_i`, `B_i`, `F_j`, and `G_j`,
/// find points `C`, `D`, and `E` such that
/// the bezier splines `A_i B_i C D E F_j G_j` have:
/// 1. Continuous first derivative at `D`
/// 2. Minimum discontinuity in the second derivative at `D`.
/// 3. Consistent with the above, minimum discontinuity in the third derivative at `D`.
pub fn connect_bezier(
    ab: impl Iterator<Item = (f64, f64)>,
    fg: impl Iterator<Item = (f64, f64)>,
) -> (f64, f64, f64) {
    let a: f64;
    let b: f64;
    let f: f64;
    let g: f64;

    {
        let mut a_sum = 0.;
        let mut b_sum = 0.;
        let mut ab_count = 0;

        for (ai, bi) in ab {
            a_sum += ai;
            b_sum += bi;
            ab_count += 1;
        }

        a = a_sum / f64::from(ab_count);
        b = b_sum / f64::from(ab_count);
    }

    {
        let mut f_sum = 0.;
        let mut g_sum = 0.;
        let mut fg_count = 0;

        for (fi, gi) in fg {
            f_sum += fi;
            g_sum += gi;
            fg_count += 1;
        }

        f = f_sum / f64::from(fg_count);
        g = g_sum / f64::from(fg_count);
    }

    // Continuity of first derivative implies `E-D = D-C`.
    // Minimum discontinuity of second derivative further implies `E-D = D-C = (F-B) / 4`.
    // Minimum discontinuity of third derivative further implies `D = ((3B-A) + (3F-G)) / 4`.

    let d = (3. * b - a + 3. * f - g) / 4.;
    let v = (f - b) / 4.;
    (d - v, d, d + v)
}
